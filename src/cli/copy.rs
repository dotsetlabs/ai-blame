//! Copy attribution notes between commits

use anyhow::{Context, Result};
use clap::Args;
use git2::Repository;

use crate::storage::notes::NotesStore;

/// Copy AI attribution from one commit to another
#[derive(Debug, Args)]
pub struct CopyNotesArgs {
    /// Source commit SHA (before rewrite)
    pub source: String,

    /// Target commit SHA (after rewrite)
    pub target: String,

    /// Show what would be copied without copying
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: CopyNotesArgs) -> Result<()> {
    let repo = Repository::discover(".").context("Not in a git repository")?;

    let source_oid = repo.revparse_single(&args.source)?.peel_to_commit()?.id();
    let target_oid = repo.revparse_single(&args.target)?.peel_to_commit()?.id();

    let store = NotesStore::new(&repo)?;

    if !store.has_attribution(source_oid) {
        println!("Source commit {} has no attribution.", &args.source);
        return Ok(());
    }

    let source_short = &args.source[..8.min(args.source.len())];
    let target_short = &args.target[..8.min(args.target.len())];

    if args.dry_run {
        println!(
            "Would copy attribution: {} -> {}",
            source_short, target_short
        );
        return Ok(());
    }

    store.copy_attribution(source_oid, target_oid)?;
    println!("Copied attribution: {} -> {}", source_short, target_short);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_notes_args_structure() {
        let args = CopyNotesArgs {
            source: "abc123".to_string(),
            target: "def456".to_string(),
            dry_run: false,
        };

        assert_eq!(args.source, "abc123");
        assert_eq!(args.target, "def456");
        assert!(!args.dry_run);
    }

    #[test]
    fn test_copy_notes_args_dry_run() {
        let args = CopyNotesArgs {
            source: "abc123".to_string(),
            target: "def456".to_string(),
            dry_run: true,
        };

        assert!(args.dry_run);
    }

    #[test]
    fn test_short_sha_truncation() {
        // Test that short SHAs are handled correctly
        let short = "abc";
        let result = &short[..8.min(short.len())];
        assert_eq!(result, "abc");

        let long = "abc123def456789";
        let result = &long[..8.min(long.len())];
        assert_eq!(result, "abc123de");

        let exact = "12345678";
        let result = &exact[..8.min(exact.len())];
        assert_eq!(result, "12345678");
    }
}
