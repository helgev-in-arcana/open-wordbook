use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlashcardConfig {
    pub alpha: f32,
    pub weight_mean: f32,
    pub weight_variance: f32,
    pub time_decay_factor: f32,
    pub new_weight_initial_value: f32,
}

impl Default for FlashcardConfig {
    fn default() -> Self {
        Self {
            alpha: 0.3,
            weight_mean: 1.0,
            weight_variance: 1.0,
            time_decay_factor: 0.00001,
            new_weight_initial_value: 3.0,
        }
    }
}

pub fn load_config(config_dir: &Path) -> Result<FlashcardConfig, String> {
    let config_path = config_dir.join("config.json");

    if config_path.exists() {
        let contents =
            fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config: {}", e))?;
        let config =
            serde_json::from_str(&contents).map_err(|e| format!("Failed to parse config: {}", e))?;
        Ok(config)
    } else {
        // Create default config
        let default_config = FlashcardConfig::default();
        save_config(config_dir, &default_config)?;
        Ok(default_config)
    }
}

pub fn save_config(config_dir: &Path, config: &FlashcardConfig) -> Result<(), String> {
    if !config_dir.exists() {
        fs::create_dir_all(config_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }

    let config_path = config_dir.join("config.json");
    let contents = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(config_path, contents).map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_load_and_save_config() {
        let temp_dir = env::temp_dir().join("openwordbook_test_config");
        if temp_dir.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
        }

        // Test loading when no config exists (should create default)
        let config1 = load_config(&temp_dir).unwrap();
        assert_eq!(config1.alpha, 0.3); // Check a default value

        let config_path = temp_dir.join("config.json");
        assert!(config_path.exists());

        // Test saving and loading back modified config
        let mut modified_config = config1.clone();
        modified_config.alpha = 0.8;
        modified_config.new_weight_initial_value = 5.0;

        save_config(&temp_dir, &modified_config).unwrap();

        let config2 = load_config(&temp_dir).unwrap();
        assert_eq!(config2.alpha, 0.8);
        assert_eq!(config2.new_weight_initial_value, 5.0);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
