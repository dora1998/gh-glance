use anyhow::{Context, Result};
use std::process::Command;
use std::path::Path;

pub struct Git;

impl Git {
    pub fn new() -> Self {
        Self
    }

    fn get_repo_root(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("Failed to get repository root")?;

        if !output.status.success() {
            anyhow::bail!("Failed to get repository root: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(String::from_utf8(output.stdout)
            .context("Invalid UTF-8")?
            .trim()
            .to_string())
    }

    pub fn exists_worktree(&self, path: &str) -> bool {
        let worktrees = self.list_worktrees().unwrap_or_default();
        let repo_root = self.get_repo_root().unwrap_or_default();
        let target_path = Path::new(&repo_root).join(path);
        worktrees.iter().any(|w| Path::new(w) == target_path)
    }

    pub fn add_worktree(&self, branch: &str, path: &str) -> Result<()> {
        if self.exists_worktree(path) {
            return Ok(());
        }

        println!("Adding worktree for branch '{}' at '{}'", branch, path);
        let repo_root = self.get_repo_root()?;
        let output = Command::new("git")
            .args(["worktree", "add", path, branch])
            .current_dir(&repo_root)
            .output()
            .context("Failed to add worktree")?;

        if !output.status.success() {
            anyhow::bail!("Failed to add worktree: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    pub fn remove_worktree(&self, path: &str) -> Result<()> {
        if !self.exists_worktree(path) {
            anyhow::bail!("Worktree not found at: {}", path);
        }

        let repo_root = self.get_repo_root()?;
        Command::new("git")
            .args(["worktree", "remove", path])
            .current_dir(&repo_root)
            .status()
            .context("Failed to remove worktree")?;
        Ok(())
    }

    pub fn list_worktrees(&self) -> Result<Vec<String>> {
        let repo_root = self.get_repo_root()?;
        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(&repo_root)
            .output()
            .context("Failed to list worktrees")?;

        let output = String::from_utf8(output.stdout).context("Invalid UTF-8")?;
        Ok(output
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .map(|line| line[9..].to_string())
            .collect())
    }

    pub fn pull(&self, path: &str, branch: &str, mode: &str) -> Result<()> {
        let repo_root = self.get_repo_root()?;
        let full_path = Path::new(&repo_root).join(path);
        if !full_path.exists() {
            return Ok(());
        }

        // 追跡ブランチの取得
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
            .current_dir(&full_path)
            .output()
            .context("Failed to get upstream branch")?;

        if !output.status.success() {
            // 追跡ブランチが設定されていない場合は何もしない
            return Ok(());
        }

        let upstream = String::from_utf8(output.stdout)
            .context("Invalid UTF-8")?
            .trim()
            .to_string();

        // 追跡ブランチの存在確認
        let output = Command::new("git")
            .args(["rev-parse", "--verify", &upstream])
            .current_dir(&full_path)
            .output()
            .context("Failed to check remote branch")?;

        if !output.status.success() {
            // リモートブランチが存在しない場合は何もしない
            return Ok(());
        }

        match mode {
            "default" => {
                Command::new("git")
                    .args(["pull"])
                    .current_dir(&full_path)
                    .status()
                    .context("Failed to pull changes")?;
            }
            "force" => {
                Command::new("git")
                    .args(["fetch"])
                    .current_dir(&full_path)
                    .status()
                    .context("Failed to fetch changes")?;

                Command::new("git")
                    .args(["reset", "--hard", &format!("origin/{}", branch)])
                    .current_dir(&full_path)
                    .status()
                    .context("Failed to reset branch")?;
            }
            "off" => {
                // 何もしない
                return Ok(());
            }
            _ => {
                anyhow::bail!("Invalid auto_pull mode: {}", mode);
            }
        }
        Ok(())
    }

    pub fn is_branch_merged(&self, path: &str) -> Result<bool> {
        let repo_root = self.get_repo_root()?;
        let full_path = Path::new(&repo_root).join(path);

        // mainブランチの最新コミットを取得
        let output = Command::new("git")
            .args(["rev-parse", "main"])
            .current_dir(&full_path)
            .output()
            .context("Failed to get main branch commit")?;

        if !output.status.success() {
            return Ok(false);
        }

        let main_commit = String::from_utf8(output.stdout)?.trim().to_string();

        // 現在のブランチのコミットがmainブランチに含まれているかチェック
        let output = Command::new("git")
            .args(["merge-base", "--is-ancestor", "HEAD", &main_commit])
            .current_dir(&full_path)
            .status()
            .context("Failed to check if branch is merged")?;

        Ok(output.success())
    }

    pub fn can_fast_forward(&self, path: &str) -> Result<bool> {
        let repo_root = self.get_repo_root()?;
        let full_path = Path::new(&repo_root).join(path);

        // リモートの最新状態を取得
        Command::new("git")
            .args(["fetch"])
            .current_dir(&full_path)
            .status()
            .context("Failed to fetch")?;

        // 現在のブランチの追跡ブランチを取得
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
            .current_dir(&full_path)
            .output()
            .context("Failed to get upstream branch")?;

        if !output.status.success() {
            return Ok(false);
        }

        let upstream = String::from_utf8(output.stdout)?.trim().to_string();

        // fast-forwardマージ可能かチェック
        let output = Command::new("git")
            .args(["merge-base", "--is-ancestor", "HEAD", &upstream])
            .current_dir(&full_path)
            .status()
            .context("Failed to check if can fast-forward")?;

        Ok(output.success())
    }
} 