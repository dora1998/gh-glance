use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub base: BaseConfig,
    #[serde(default)]
    pub tasks: HashMap<String, TaskConfig>,
}

#[derive(Debug, Deserialize)]
pub struct BaseConfig {
    #[serde(default)]
    pub prepare_task: String,
    #[serde(default = "default_auto_pull")]
    pub auto_pull: String,
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,
    #[serde(default = "default_auto_checkout")]
    pub auto_checkout: bool,
}

#[derive(Debug, Deserialize)]
pub struct TaskConfig {
    pub run: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Path::new(".gh-glance.toml");
        
        // 設定ファイルが存在しない場合はデフォルト設定を使用
        if !config_path.exists() {
            return Ok(Self {
                base: BaseConfig::default(),
                tasks: HashMap::new(),
            });
        }

        let content = fs::read_to_string(config_path)
            .context("Failed to read config file")?;
        toml::from_str(&content).context("Failed to parse config file")
    }

    pub fn get_task(&self, name: &str) -> Option<&TaskConfig> {
        self.tasks.get(name)
    }
}

impl Default for BaseConfig {
    fn default() -> Self {
        Self {
            prepare_task: String::new(),
            auto_pull: default_auto_pull(),
            worktree_dir: default_worktree_dir(),
            auto_checkout: default_auto_checkout(),
        }
    }
}

fn default_auto_pull() -> String {
    "default".to_string()
}

fn default_worktree_dir() -> String {
    ".worktree".to_string()
}

fn default_auto_checkout() -> bool {
    true
} 