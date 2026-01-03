//! Android-specific platform functionality
//!
//! Handles storage permissions using JNI to call Android APIs.
//! See ADR-0009 for details on the permission requirements.

use super::StoragePermissionStatus;
use jni::JNIEnv;
use jni::objects::{JObject, JValue};

const READ_EXTERNAL_STORAGE: &str = "android.permission.READ_EXTERNAL_STORAGE";
const PERMISSION_GRANTED: i32 = 0; // PackageManager.PERMISSION_GRANTED

/// Get the Android SDK version (Build.VERSION.SDK_INT)
fn get_sdk_version(env: &mut JNIEnv) -> Result<i32, jni::errors::Error> {
    let build_version = env.find_class("android/os/Build$VERSION")?;
    let sdk_int = env.get_static_field(build_version, "SDK_INT", "I")?;
    sdk_int.i()
}

/// Check if we have READ_EXTERNAL_STORAGE permission (Android 10 and below)
fn check_read_storage_permission(
    env: &mut JNIEnv,
    context: &JObject,
) -> Result<bool, jni::errors::Error> {
    // ContextCompat.checkSelfPermission(context, permission)
    let context_compat = env.find_class("androidx/core/content/ContextCompat")?;
    let permission = env.new_string(READ_EXTERNAL_STORAGE)?;

    let result = env.call_static_method(
        context_compat,
        "checkSelfPermission",
        "(Landroid/content/Context;Ljava/lang/String;)I",
        &[JValue::Object(context), JValue::Object(&permission.into())],
    )?;

    Ok(result.i()? == PERMISSION_GRANTED)
}

/// Check if we have MANAGE_EXTERNAL_STORAGE permission (Android 11+)
/// Uses Environment.isExternalStorageManager()
fn check_manage_storage_permission(env: &mut JNIEnv) -> Result<bool, jni::errors::Error> {
    let environment = env.find_class("android/os/Environment")?;
    let result = env.call_static_method(environment, "isExternalStorageManager", "()Z", &[])?;
    result.z()
}

/// Open the system settings for "All files access" permission (Android 11+)
fn open_manage_storage_settings(
    env: &mut JNIEnv,
    context: &JObject,
) -> Result<(), jni::errors::Error> {
    // Create intent: new Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION)
    let settings = env.find_class("android/provider/Settings")?;
    let action = env.get_static_field(
        settings,
        "ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION",
        "Ljava/lang/String;",
    )?;

    let intent_class = env.find_class("android/content/Intent")?;
    let intent = env.new_object(
        intent_class,
        "(Ljava/lang/String;)V",
        &[JValue::Object(&action.l()?)],
    )?;

    // Add FLAG_ACTIVITY_NEW_TASK
    let flag_new_task: i32 = 0x10000000; // Intent.FLAG_ACTIVITY_NEW_TASK
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(flag_new_task)],
    )?;

    // context.startActivity(intent)
    env.call_method(
        context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )?;

    Ok(())
}

/// Open app settings page for legacy permission request (Android 10 and below)
fn open_app_settings(env: &mut JNIEnv, context: &JObject) -> Result<(), jni::errors::Error> {
    // Create intent: new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS)
    let settings = env.find_class("android/provider/Settings")?;
    let action = env.get_static_field(
        settings,
        "ACTION_APPLICATION_DETAILS_SETTINGS",
        "Ljava/lang/String;",
    )?;

    let intent_class = env.find_class("android/content/Intent")?;
    let intent = env.new_object(
        intent_class,
        "(Ljava/lang/String;)V",
        &[JValue::Object(&action.l()?)],
    )?;

    // Set data to our package URI: Uri.parse("package:" + packageName)
    let package_name_jvalue =
        env.call_method(context, "getPackageName", "()Ljava/lang/String;", &[])?;
    let package_name_jstring: jni::objects::JString = package_name_jvalue.l()?.into();
    let package_name_str = env.get_string(&package_name_jstring)?;
    let package_name = package_name_str.to_str().unwrap_or("");
    let uri_string = env.new_string(format!("package:{}", package_name))?;

    let uri_class = env.find_class("android/net/Uri")?;
    let uri = env.call_static_method(
        uri_class,
        "parse",
        "(Ljava/lang/String;)Landroid/net/Uri;",
        &[JValue::Object(&uri_string.into())],
    )?;

    env.call_method(
        &intent,
        "setData",
        "(Landroid/net/Uri;)Landroid/content/Intent;",
        &[JValue::Object(&uri.l()?)],
    )?;

    // Add FLAG_ACTIVITY_NEW_TASK
    let flag_new_task: i32 = 0x10000000;
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(flag_new_task)],
    )?;

    // context.startActivity(intent)
    env.call_method(
        context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )?;

    Ok(())
}

/// Helper to run JNI operations with proper error handling
fn with_jni<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&mut JNIEnv, JObject) -> Result<T, jni::errors::Error>,
{
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    match f(&mut env, context) {
        Ok(result) => Some(result),
        Err(e) => {
            log::error!("JNI error: {e}");
            None
        }
    }
}

/// Check if the app has storage permission to read external folders.
pub fn check_storage_permission() -> StoragePermissionStatus {
    with_jni(|env, context| {
        let sdk_version = get_sdk_version(env)?;
        log::info!("Android SDK version: {sdk_version}");

        if sdk_version >= 30 {
            // Android 11+ (API 30): Need MANAGE_EXTERNAL_STORAGE
            if check_manage_storage_permission(env)? {
                Ok(StoragePermissionStatus::Granted)
            } else {
                Ok(StoragePermissionStatus::NeedsSettingsIntent)
            }
        } else {
            // Android 10 and below: Need READ_EXTERNAL_STORAGE
            if check_read_storage_permission(env, &context)? {
                Ok(StoragePermissionStatus::Granted)
            } else {
                Ok(StoragePermissionStatus::Denied)
            }
        }
    })
    .unwrap_or(StoragePermissionStatus::Denied)
}

/// Request storage permission by opening the appropriate settings page.
///
/// On Android 11+, opens "All files access" settings.
/// On Android 10 and below, opens app settings where user can grant storage permission.
///
/// Returns `true` if the settings page was opened successfully.
pub fn request_storage_permission() -> bool {
    with_jni(|env, context| {
        let sdk_version = get_sdk_version(env)?;

        if sdk_version >= 30 {
            open_manage_storage_settings(env, &context)
        } else {
            open_app_settings(env, &context)
        }
    })
    .is_some()
}
