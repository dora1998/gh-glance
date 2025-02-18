mod config;
mod git;
mod github;
mod task;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::Config;
use git::Git;
use github::GitHub;
use task::TaskRunner;
use std::path::Path;
use std::io::{self, Write};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// PR number or branch name
    #[arg(global = true)]
    target: Option<String>,

    /// Task name to run
    #[arg(global = true)]
    task: Option<String>,

    /// Command to run after --
    #[arg(last = true)]
    command_args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run tasks in PR's worktree
    Run {
        /// PR number or branch name
        target: String,
        /// Task name
        task: String,
    },
    /// Get PR's worktree directory
    Dir {
        /// PR number or branch name
        target: String,
    },
    /// Move to PR's worktree
    Checkout {
        /// PR number or branch name
        target: String,
    },
    /// Remove PR's worktree
    Rm {
        /// PR number or branch name
        target: String,
    },
    /// Remove all merged branch's worktrees
    Clean,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let git = Git::new();
    let github = GitHub::new().await?;

    // Handle direct command with --
    if !cli.command_args.is_empty() {
        if let Some(target) = cli.target {
            run_direct_command(&target, &cli.command_args)?;
            return Ok(());
        }
    }

    match cli.command {
        Some(Commands::Run { target, task }) => {
            run_task(&target, &task).await?;
        }
        Some(Commands::Dir { target }) => {
            get_worktree_dir(&git, &github, &target).await?;
        }
        Some(Commands::Checkout { target }) => {
            checkout_target(&git, &github, &target).await?;
        }
        Some(Commands::Rm { target }) => {
            remove_target(&git, &target)?;
        }
        Some(Commands::Clean) => {
            clean_worktrees(&git, &github).await?;
        }
        None => {
            if let (Some(target), Some(task)) = (cli.target, cli.task) {
                run_task(&target, &task).await?;
            }
        }
    }

    Ok(())
}

async fn run_task(target: &str, task: &str) -> Result<()> {
    let config = Config::load()?;
    let workdir = Path::new(&config.base.worktree_dir).join(target);
    let git = Git::new();
    let github = GitHub::new().await?;
    
    let workdir_str = workdir.to_str()
        .with_context(|| format!("Invalid characters in worktree path: {}", workdir.display()))?;

    // auto_checkoutが有効な場合は、ワークツリーが存在しない場合に自動的にチェックアウトする
    if config.base.auto_checkout && !git.exists_worktree(workdir_str) {
        checkout_target(&git, &github, target).await?;
    } else if !git.exists_worktree(workdir_str) {
        anyhow::bail!("Worktree not found at: {}. Please run 'checkout' first.", workdir.display());
    }

    let runner = TaskRunner::new(&config, workdir_str.to_string());

    // タスクが定義されていない場合はエラーを返す
    if config.get_task(task).is_some() {
        runner.run(task)
    } else {
        anyhow::bail!("Task '{}' is not defined in the configuration file", task)
    }
}

fn run_direct_command(target: &str, args: &[String]) -> Result<()> {
    let config = Config::load()?;
    let workdir = Path::new(&config.base.worktree_dir).join(target);
    let git = Git::new();
    
    let workdir_str = workdir.to_str()
        .with_context(|| format!("Invalid characters in worktree path: {}", workdir.display()))?;

    if !git.exists_worktree(workdir_str) {
        anyhow::bail!("Worktree not found at: {}. Please run 'checkout' first.", workdir.display());
    }

    let runner = TaskRunner::new(&config, workdir_str.to_string());
    runner.execute_command(&args.join(" "))
}

async fn checkout_target(git: &Git, github: &GitHub, target: &str) -> Result<()> {
    let config = Config::load()?;
    let workdir = Path::new(&config.base.worktree_dir).join(target);
    let workdir_str = workdir.to_str()
        .with_context(|| format!("Invalid characters in worktree path: {}", workdir.display()))?;
    
    // 既存のワークツリーをチェック
    if git.exists_worktree(workdir_str) {
        println!("Worktree already exists at: {}", workdir.display());
        return Ok(());
    }
    
    // PRの場合はブランチ名を取得
    let branch = if let Ok(pr_number) = target.parse::<u64>() {
        github.get_pr_branch(pr_number).await?
    } else {
        target.to_string()
    };

    git.add_worktree(&branch, workdir_str)?;
    
    // auto_pullの設定に応じてプル
    if config.base.auto_pull != "off" {
        git.pull(workdir_str, &branch, &config.base.auto_pull)?;
    }

    Ok(())
}

fn remove_target(git: &Git, target: &str) -> Result<()> {
    let config = Config::load()?;
    let workdir = Path::new(&config.base.worktree_dir).join(target);
    let workdir_str = workdir.to_str()
        .with_context(|| format!("Invalid characters in worktree path: {}", workdir.display()))?;
    git.remove_worktree(workdir_str)
}

async fn clean_worktrees(git: &Git, github: &GitHub) -> Result<()> {
    let config = Config::load()?;
    let base_dir = Path::new(&config.base.worktree_dir);
    let worktrees = git.list_worktrees()?;
    let mut to_remove = Vec::new();

    // 削除対象のworktreeを収集
    for worktree in worktrees {
        let worktree_path = Path::new(&worktree);
        if !worktree_path.starts_with(base_dir) {
            continue;
        }

        // PRの場合はGitHubのAPIでマージ状態を確認
        let is_merged = if let Some(pr_number) = github.extract_pr_number_from_worktree(&worktree) {
            github.is_pr_merged(pr_number).await?
        } else {
            // PRでない場合は従来通りgitコマンドで確認
            git.is_branch_merged(&worktree)?
        };

        // マージ済みかつfast-forwardでマージ可能なworktreeのみを対象とする
        if is_merged && git.can_fast_forward(&worktree)? {
            to_remove.push(worktree);
        }
    }

    if to_remove.is_empty() {
        println!("No worktrees to remove.");
        return Ok(());
    }

    // 削除対象の一覧を表示
    println!("\nThe following worktrees will be removed:");
    for worktree in &to_remove {
        println!("  - {}", worktree);
    }

    // ユーザーに確認
    print!("\nDo you want to continue? [y/N]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" {
        // 確認が取れたら削除を実行
        for worktree in to_remove {
            println!("Removing worktree: {}", worktree);
            git.remove_worktree(&worktree)?;
        }
        println!("\nRemoval completed.");
    } else {
        println!("\nOperation cancelled.");
    }

    Ok(())
}

async fn get_worktree_dir(git: &Git, github: &GitHub, target: &str) -> Result<()> {
    let config = Config::load()?;
    let repo_root = git.get_repo_root()?;
    let workdir = Path::new(&repo_root).join(&config.base.worktree_dir).join(target);
    let workdir_str = workdir.to_str()
        .with_context(|| format!("Invalid characters in worktree path: {}", workdir.display()))?;
    
    // auto_checkoutが有効な場合は、ワークツリーが存在しない場合に自動的にチェックアウトする
    if config.base.auto_checkout && !git.exists_worktree(workdir_str) {
        checkout_target(git, github, target).await?;
    } else if !git.exists_worktree(workdir_str) {
        anyhow::bail!("Worktree not found at: {}. Please run 'checkout' first.", workdir.display());
    }

    println!("{}", workdir_str);
    Ok(())
} 