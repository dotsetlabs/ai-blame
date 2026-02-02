# copy-notes

Copy AI attribution from one commit to another.

## Usage

```bash
whogitit copy-notes <SOURCE> <TARGET> [OPTIONS]
```

## Description

The `copy-notes` command copies attribution data from a source commit to a target commit. This is useful for:

- Recovering attribution after cherry-pick operations
- Manually fixing attribution after complex rebases
- Copying attribution when the post-rewrite hook wasn't installed

## Arguments

| Argument | Description |
|----------|-------------|
| `SOURCE` | Source commit SHA (the commit with attribution) |
| `TARGET` | Target commit SHA (the commit to copy attribution to) |

## Options

| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would be copied without actually copying |

## Examples

### Copy attribution after cherry-pick

Cherry-pick doesn't automatically transfer attribution:

```bash
# Cherry-pick a commit
git cherry-pick abc123

# Copy the attribution to the new commit
whogitit copy-notes abc123 HEAD
```

### Preview before copying

```bash
whogitit copy-notes abc123 def456 --dry-run
# Would copy attribution: abc123 -> def456
```

### Batch copy after rebase

If you rebased without the post-rewrite hook installed:

```bash
# Save old commits before rebase
git log --format='%H' main..HEAD > /tmp/old-commits.txt

# After rebase, save new commits
git log --format='%H' main..HEAD > /tmp/new-commits.txt

# Copy notes for each pair
paste /tmp/old-commits.txt /tmp/new-commits.txt | while read old new; do
  whogitit copy-notes "$old" "$new"
done
```

## Automatic Preservation

For most rebase and amend operations, you don't need this command. The post-rewrite hook (installed by `whogitit init`) automatically preserves attribution during:

- `git rebase`
- `git commit --amend`

Use `copy-notes` only for:
- Cherry-pick operations (not covered by post-rewrite hook)
- Repositories where post-rewrite hook wasn't installed
- Manual recovery scenarios

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (or source has no attribution) |
| 1 | Error (invalid commit, repository issues) |

## See Also

- [init](./init.md) - Install hooks including post-rewrite
- [Git Notes Storage](../../reference/git-notes.md) - How attribution is stored
- [Troubleshooting](../../appendix/troubleshooting.md) - Common issues
