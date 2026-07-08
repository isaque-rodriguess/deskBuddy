use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetConfig {
    #[serde(default = "default_slug")]
    pub slug: String,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub run_enabled: bool,
}

fn default_slug() -> String {
    "zero-2".into()
}
fn default_scale() -> f64 {
    0.7
}
fn default_true() -> bool {
    true
}

impl Default for PetConfig {
    fn default() -> Self {
        PetConfig {
            slug: default_slug(),
            scale: default_scale(),
            enabled: true,
            run_enabled: true,
        }
    }
}

impl PetConfig {
    pub fn load(path: &Path) -> PetConfig {
        let mut cfg = PetConfig::default();
        if let Ok(s) = std::fs::read_to_string(path) {
            if let Ok(loaded) = serde_json::from_str::<PetConfig>(&s) {
                cfg = loaded;
            }
        }
        cfg
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = serde_json::to_string_pretty(self).unwrap_or_default();
        std::fs::write(path, s)
    }

    pub fn migrate_from_legacy(legacy_path: &Path, new_path: &Path) -> PetConfig {
        let cfg = PetConfig::load(legacy_path);
        if legacy_path.exists() {
            let _ = cfg.save(new_path);
        }
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn defaults() {
        let c = PetConfig::default();
        assert_eq!(c.slug, "zero-2");
        assert!((c.scale - 0.7).abs() < 1e-9);
        assert!(c.enabled);
        assert!(c.run_enabled);
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("pet_config.json");
        let cfg = PetConfig {
            slug: "codax".into(),
            scale: 1.1,
            enabled: false,
            run_enabled: true,
        };
        cfg.save(&p).unwrap();
        let loaded = PetConfig::load(&p);
        assert_eq!(loaded.slug, "codax");
        assert!((loaded.scale - 1.1).abs() < 1e-9);
        assert!(!loaded.enabled);
        assert!(loaded.run_enabled);
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let p = std::path::PathBuf::from("/nonexistent/deskbuddy/missing.json");
        let c = PetConfig::load(&p);
        assert_eq!(c.slug, "zero-2");
        assert!((c.scale - 0.7).abs() < 1e-9);
    }

    #[test]
    fn load_partial_json_uses_defaults_for_missing() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("pet_config.json");
        std::fs::write(&p, r#"{"slug":"theo-3"}"#).unwrap();
        let c = PetConfig::load(&p);
        assert_eq!(c.slug, "theo-3");
        assert!((c.scale - 0.7).abs() < 1e-9, "scale should default");
        assert!(c.enabled, "enabled should default");
    }

    #[test]
    fn load_invalid_json_returns_defaults() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("pet_config.json");
        std::fs::write(&p, "not json{").unwrap();
        let c = PetConfig::load(&p);
        assert_eq!(c.slug, "zero-2");
    }

    #[test]
    fn migrate_from_legacy_copies() {
        let dir = TempDir::new().unwrap();
        let legacy = dir.path().join("pet_config.json");
        let newp = dir.path().join("store").join("config.json");
        let cfg = PetConfig {
            slug: "prime-lab".into(),
            scale: 0.9,
            enabled: true,
            run_enabled: false,
        };
        cfg.save(&legacy).unwrap();
        let migrated = PetConfig::migrate_from_legacy(&legacy, &newp);
        assert_eq!(migrated.slug, "prime-lab");
        assert!(newp.exists(), "new path should be written");
        let reloaded = PetConfig::load(&newp);
        assert_eq!(reloaded.slug, "prime-lab");
        assert!(!reloaded.run_enabled);
    }
}
