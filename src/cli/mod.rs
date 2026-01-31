pub mod blame;
pub mod output;
pub mod prompt;
pub mod show;
pub mod summary;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::capture::hook;

/// AI-aware git blame tool for tracking AI-generated code
#[derive(Debug, Parser)]
#[command(name = "ai-blame")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Show AI attribution for each line of a file
    Blame(blame::BlameArgs),

    /// View the prompt that generated specific lines
    Prompt(prompt::PromptArgs),

    /// Show AI attribution summary for a commit
    Show(show::ShowArgs),

    /// Generate summary for a range of commits (useful for PRs)
    Summary(summary::SummaryArgs),

    /// Capture a file change (called by Claude Code hook)
    Capture(CaptureArgs),

    /// Finalize attribution after a commit (post-commit hook)
    PostCommit,

    /// Show pending changes status
    Status,

    /// Clear pending changes without committing
    Clear,

    /// Initialize ai-blame in a git repository (installs post-commit hook)
    Init,
}

/// Capture command arguments
#[derive(Debug, clap::Args)]
pub struct CaptureArgs {
    /// Read hook input from stdin
    #[arg(long)]
    pub stdin: bool,

    /// File path (if not using stdin)
    #[arg(long)]
    pub file: Option<String>,

    /// Tool name
    #[arg(long)]
    pub tool: Option<String>,

    /// Prompt text
    #[arg(long)]
    pub prompt: Option<String>,
}

/// Run the CLI
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Blame(args) => blame::run(args),
        Commands::Prompt(args) => prompt::run(args),
        Commands::Show(args) => show::run(args),
        Commands::Summary(args) => summary::run(args),
        Commands::Capture(args) => run_capture(args),
        Commands::PostCommit => run_post_commit(),
        Commands::Status => run_status(),
        Commands::Clear => run_clear(),
        Commands::Init => run_init(),
    }
}

fn run_capture(args: CaptureArgs) -> Result<()> {
    if args.stdin {
        hook::run_capture_hook()
    } else {
        anyhow::bail!("Capture requires --stdin flag for hook input")
    }
}

fn run_post_commit() -> Result<()> {
    hook::run_post_commit_hook()
}

fn run_status() -> Result<()> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo.workdir()
        .ok_or_else(|| anyhow::anyhow!("No working directory"))?;

    let hook_handler = crate::capture::CaptureHook::new(repo_root)?;
    let status = hook_handler.status()?;

    if status.has_pending {
        println!("Pending AI attribution:");
        println!("  Session: {}", status.session_id.as_deref().unwrap_or("unknown"));
        println!("  Files: {}", status.file_count);
        println!("  Lines: {}", status.line_count);
        println!("\nRun 'git commit' to finalize attribution.");
    } else {
        println!("No pending AI attribution.");
    }

    Ok(())
}

fn run_clear() -> Result<()> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo.workdir()
        .ok_or_else(|| anyhow::anyhow!("No working directory"))?;

    let hook_handler = crate::capture::CaptureHook::new(repo_root)?;
    hook_handler.clear_pending()?;

    println!("Cleared pending AI attribution.");

    Ok(())
}

fn run_init() -> Result<()> {
    let repo = git2::Repository::discover(".")
        .context("Not in a git repository")?;
    let repo_root = repo.workdir()
        .ok_or_else(|| anyhow::anyhow!("No working directory"))?;

    // Install post-commit hook
    let hooks_dir = repo_root.join(".git/hooks");
    fs::create_dir_all(&hooks_dir)
        .context("Failed to create hooks directory")?;

    let hook_path = hooks_dir.join("post-commit");

    // Check if hook already exists
    if hook_path.exists() {
        let content = fs::read_to_string(&hook_path)?;
        if content.contains("ai-blame") {
            println!("✓ ai-blame post-commit hook already installed.");
        } else {
            // Append to existing hook
            let new_content = format!(
                "{}\n\n# ai-blame post-commit hook\nif command -v ai-blame &> /dev/null; then\n    ai-blame post-commit 2>/dev/null || true\nfi\n",
                content.trim_end()
            );
            fs::write(&hook_path, new_content)?;
            println!("✓ Added ai-blame to existing post-commit hook.");
        }
    } else {
        // Create new hook
        let hook_content = r#"#!/bin/bash
# ai-blame post-commit hook
# Attaches AI attribution notes to the commit

if command -v ai-blame &> /dev/null; then
    ai-blame post-commit 2>/dev/null || true
elif [[ -x "$HOME/.cargo/bin/ai-blame" ]]; then
    "$HOME/.cargo/bin/ai-blame" post-commit 2>/dev/null || true
fi
"#;
        fs::write(&hook_path, hook_content)?;

        // Make executable
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;

        println!("✓ Installed ai-blame post-commit hook.");
    }

    // Configure git to auto-push/fetch notes with regular push/pull
    configure_git_notes(&repo)?;

    println!("\nSetup complete! AI attribution will be tracked for commits in this repo.");
    println!("Notes will be automatically pushed/fetched with 'git push' and 'git fetch'.");
    println!("\nMake sure Claude Code hooks are configured in ~/.claude/settings.json");

    Ok(())
}

/// Configure git to automatically push and fetch ai-blame notes
fn configure_git_notes(repo: &git2::Repository) -> Result<()> {
    let mut config = repo.config()
        .context("Failed to open git config")?;

    // Check if push refspec already configured
    let push_refspec = "refs/notes/ai-blame";
    let push_configured = config
        .get_string("remote.origin.push")
        .map(|v| v.contains("ai-blame"))
        .unwrap_or(false);

    if !push_configured {
        // Use multivar to add without replacing existing push configs
        config.set_multivar("remote.origin.push", "^$", push_refspec)
            .or_else(|_| {
                // If multivar fails, try regular set (might be first entry)
                config.set_str("remote.origin.push", push_refspec)
            })
            .context("Failed to configure push refspec")?;
        println!("✓ Configured git to push ai-blame notes automatically.");
    } else {
        println!("✓ Git already configured to push ai-blame notes.");
    }

    // Check if fetch refspec already configured
    let fetch_refspec = "+refs/notes/ai-blame:refs/notes/ai-blame";
    let fetch_configured = config
        .get_string("remote.origin.fetch")
        .map(|v| v.contains("ai-blame"))
        .unwrap_or(false);

    if !fetch_configured {
        config.set_multivar("remote.origin.fetch", "^$", fetch_refspec)
            .or_else(|_| {
                config.set_str("remote.origin.fetch", fetch_refspec)
            })
            .context("Failed to configure fetch refspec")?;
        println!("✓ Configured git to fetch ai-blame notes automatically.");
    } else {
        println!("✓ Git already configured to fetch ai-blame notes.");
    }

    Ok(())
}
