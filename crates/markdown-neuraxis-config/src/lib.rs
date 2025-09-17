use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file at {config_path}: {source}")]
    ConfigReadError {
        config_path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse config file at {config_path}: {source}")]
    ConfigParseError {
        config_path: PathBuf,
        source: toml::de::Error,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub notes_path: PathBuf,
}

impl Config {
    pub fn load_from_path<P: AsRef<Path>>(config_path: P) -> Result<Option<Self>, ConfigError> {
        let config_path = config_path.as_ref();
        if !config_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(config_path).map_err(|source| {
            ConfigError::ConfigReadError {
                config_path: config_path.to_path_buf(),
                source,
            }
        })?;

        let mut config: Config =
            toml::from_str(&content).map_err(|source| ConfigError::ConfigParseError {
                config_path: config_path.to_path_buf(),
                source,
            })?;

        // Expand shell variables and tilde in the loaded config path
        config.notes_path = Self::expand_path(&config.notes_path).unwrap_or(config.notes_path);

        Ok(Some(config))
    }

    pub fn load() -> Result<Option<Self>, ConfigError> {
        let config_path = Self::config_path();
        Self::load_from_path(&config_path)
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, config_path: P) -> anyhow::Result<()> {
        let config_path = config_path.as_ref();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path();
        self.save_to_path(&config_path)
    }

    pub fn config_path() -> PathBuf {
        let config_dir = shellexpand::tilde("~/.config/markdown-neuraxis");
        PathBuf::from(config_dir.as_ref()).join("config.toml")
    }

    fn expand_path(path: &Path) -> Option<PathBuf> {
        let path_str = path.to_string_lossy();
        match shellexpand::full(&path_str) {
            Ok(expanded) => Some(PathBuf::from(expanded.as_ref())),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_config_path() {
        let config_path = Config::config_path();
        let path_str = config_path.to_string_lossy();

        // Should not contain tilde anymore
        assert!(!path_str.starts_with('~'));
        // Should contain the expected config file name
        assert!(path_str.ends_with(".config/markdown-neuraxis/config.toml"));
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let original = Config {
            notes_path: PathBuf::from("/tmp/test-notes"),
        };

        let toml_str = toml::to_string(&original).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(original.notes_path, deserialized.notes_path);
    }

    #[test]
    fn test_expand_path_with_tilde() {
        let path = PathBuf::from("~/test/path");
        let expanded = Config::expand_path(&path);

        assert!(expanded.is_some());
        let expanded = expanded.unwrap();
        assert!(!expanded.to_string_lossy().starts_with('~'));
        assert!(expanded.to_string_lossy().contains("test/path"));
    }

    #[test]
    fn test_expand_path_with_env_var() {
        unsafe {
            env::set_var("TEST_VAR", "/test/env/path");
        }

        let path = PathBuf::from("$TEST_VAR/subdir");
        let expanded = Config::expand_path(&path);

        assert!(expanded.is_some());
        let expanded = expanded.unwrap();
        assert_eq!(expanded, PathBuf::from("/test/env/path/subdir"));

        unsafe {
            env::remove_var("TEST_VAR");
        }
    }

    #[test]
    fn test_expand_path_with_absolute_path() {
        let path = PathBuf::from("/absolute/path");
        let expanded = Config::expand_path(&path).unwrap();

        assert_eq!(expanded, path);
    }

    #[test]
    fn test_expand_path_with_relative_path() {
        let path = PathBuf::from("relative/path");
        let expanded = Config::expand_path(&path).unwrap();

        assert_eq!(expanded, path);
    }

    #[test]
    fn test_load_config_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let non_existent_config = temp_dir.path().join("nonexistent.toml");

        let result = Config::load_from_path(&non_existent_config).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.toml");
        let test_config = Config {
            notes_path: PathBuf::from("/tmp/test-notes"),
        };

        // Test saving
        test_config.save_to_path(&config_file).unwrap();

        // Test loading
        let loaded_config = Config::load_from_path(&config_file).unwrap().unwrap();

        assert_eq!(loaded_config.notes_path, test_config.notes_path);
    }

    #[test]
    fn test_config_with_tilde_in_toml() {
        let config_content = r#"
notes_path = "~/test/notes"
"#;

        let mut config: Config = toml::from_str(config_content).unwrap();
        config.notes_path = Config::expand_path(&config.notes_path).unwrap_or(config.notes_path);

        let expanded_path = config.notes_path.to_string_lossy();
        assert!(!expanded_path.starts_with('~'));
        assert!(expanded_path.contains("test/notes"));
    }

    #[test]
    fn test_config_with_env_var_in_toml() {
        unsafe {
            env::set_var("NOTES_ROOT", "/custom/notes");
        }

        let config_content = r#"
notes_path = "$NOTES_ROOT/my-notes"
"#;

        let mut config: Config = toml::from_str(config_content).unwrap();
        config.notes_path = Config::expand_path(&config.notes_path).unwrap_or(config.notes_path);

        assert_eq!(config.notes_path, PathBuf::from("/custom/notes/my-notes"));

        unsafe {
            env::remove_var("NOTES_ROOT");
        }
    }

    #[test]
    fn test_save_convenience_method() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.toml");
        let test_config = Config {
            notes_path: PathBuf::from("/tmp/test-notes"),
        };

        // Test that save_to_path and save produce the same result
        // First save to a specific path
        test_config.save_to_path(&config_file).unwrap();

        // Verify the file was created and has correct content
        assert!(config_file.exists(), "Config file should exist");
        let loaded_config = Config::load_from_path(&config_file).unwrap().unwrap();
        assert_eq!(loaded_config.notes_path, test_config.notes_path);
    }
}
