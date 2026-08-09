use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_BACKEND_URL: &str = "http://127.0.0.1:25808";
const CONFIG_FILE_NAME: &str = "aioncore-gui.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GuiConfig {
    #[serde(default = "default_backend_url")]
    pub backend_url: String,
    #[serde(default)]
    pub auto_start_backend: bool,
    #[serde(default)]
    pub data_dir: Option<PathBuf>,
    #[serde(default)]
    pub work_dir: Option<PathBuf>,
    #[serde(default)]
    pub window: WindowPreferences,
    #[serde(default)]
    pub theme: ThemePreferences,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WindowPreferences {
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub maximized: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ThemePreferences {
    #[serde(default)]
    pub name: Option<String>,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            backend_url: default_backend_url(),
            auto_start_backend: false,
            data_dir: None,
            work_dir: None,
            window: WindowPreferences::default(),
            theme: ThemePreferences::default(),
        }
    }
}

pub fn load_gui_config() -> Result<GuiConfig> {
    let path = config_path()?;
    if !path.exists() {
        let config = GuiConfig::default();
        save_gui_config(&path, &config)?;
        return Ok(config);
    }

    let content =
        std::fs::read_to_string(&path).with_context(|| format!("failed to read GUI config at {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("invalid GUI config at {}", path.display()))
}

fn save_gui_config(path: &std::path::Path, config: &GuiConfig) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("GUI config path has no parent directory"))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create GUI config directory at {}", parent.display()))?;

    let content = serde_json::to_string_pretty(config).context("failed to serialize GUI config")?;
    std::fs::write(path, content).with_context(|| format!("failed to write GUI config at {}", path.display()))
}

pub fn apply_backend_url_override(config: &mut GuiConfig) -> Result<()> {
    let mut args = std::env::args().skip(1);

    while let Some(argument) = args.next() {
        if argument != "--backend-url" {
            continue;
        }

        let value = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("--backend-url requires a value"))?;
        config.backend_url = validate_backend_url(value)?;
    }

    config.backend_url = validate_backend_url(config.backend_url.clone())?;
    Ok(())
}

fn config_path() -> Result<PathBuf> {
    let base_dir = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("unable to determine the GUI config directory"))?;
    Ok(base_dir.join("aioncore-gui").join(CONFIG_FILE_NAME))
}

fn default_backend_url() -> String {
    DEFAULT_BACKEND_URL.to_string()
}

fn validate_backend_url(value: String) -> Result<String> {
    let url = reqwest::Url::parse(&value).context("backend_url must be an absolute URL")?;

    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        anyhow::bail!("backend_url must use http or https and include a host");
    }

    Ok(url.to_string().trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::GuiConfig;

    #[test]
    fn default_config_uses_loopback_backend() {
        let config = GuiConfig::default();

        assert_eq!(config.backend_url, "http://127.0.0.1:25808");
        assert!(!config.auto_start_backend);
        assert!(config.data_dir.is_none());
        assert!(config.work_dir.is_none());
    }
}
