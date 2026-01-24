# Keep UniFFI generated classes
-keep class uniffi.markdown_neuraxis_ffi.** { *; }

# Keep JNA classes
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
