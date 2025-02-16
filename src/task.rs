use crate::config::Config;
use anyhow::{Context, Result};
use std::process::Command;

pub struct TaskRunner<'a> {
    config: &'a Config,
    workdir: String,
}

impl<'a> TaskRunner<'a> {
    pub fn new(config: &'a Config, workdir: String) -> Self {
        Self { config, workdir }
    }

    pub fn run(&self, task_name: &str) -> Result<()> {
        let task = self.config.get_task(task_name)
            .context(format!("Task '{}' not found", task_name))?;

        // 準備タスクの実行（もし設定されていれば）
        if task_name != self.config.base.prepare_task {
            if let Some(prepare_task) = self.config.get_task(&self.config.base.prepare_task) {
                self.execute_command(&prepare_task.run)?;
            }
        }

        // メインタスクの実行
        self.execute_command(&task.run)?;

        Ok(())
    }

    pub fn execute_command(&self, cmd: &str) -> Result<()> {
        let status = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", cmd])
                .current_dir(&self.workdir)
                .status()
        } else {
            Command::new("sh")
                .args(["-c", cmd])
                .current_dir(&self.workdir)
                .status()
        }.context("Failed to execute command")?;

        if !status.success() {
            anyhow::bail!("Command failed with exit code: {}", status);
        }

        Ok(())
    }
} 