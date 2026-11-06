use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::git;

pub struct Worktree {
    pub path: PathBuf,
    pub created: bool,
}

/// Returns the worktree for the ticket, creating it when missing.
pub fn ensure(ticket: &str) -> Result<Worktree> {
    if let Some(path) = existing(ticket)? {
        return Ok(Worktree {
            path,
            created: false,
        });
    }

    let path = target_path(ticket)?;
    let path = path.to_str().context("worktree path is not valid UTF-8")?;

    if git::succeeds(&["show-ref", "--verify", "--quiet", &local_ref(ticket)])? {
        git::run(&["worktree", "add", path, ticket])?;
    } else {
        git::run(&["fetch", "origin"])?;
        let start = dev_start()?;
        git::run(&["worktree", "add", "-b", ticket, path, &start])?;
    }

    Ok(Worktree {
        path: PathBuf::from(path),
        created: true,
    })
}

fn existing(branch: &str) -> Result<Option<PathBuf>> {
    let listing = git::run(&["worktree", "list", "--porcelain"])?;
    let mut path = None;
    for line in listing.lines() {
        if let Some(value) = line.strip_prefix("worktree ") {
            path = Some(PathBuf::from(value));
        } else if line == format!("branch {}", local_ref(branch)) {
            return Ok(path);
        }
    }
    Ok(None)
}

fn target_path(ticket: &str) -> Result<PathBuf> {
    if git::run(&["rev-parse", "--is-bare-repository"])? == "true" {
        let git_dir = PathBuf::from(git::run(&["rev-parse", "--absolute-git-dir"])?);
        let parent = git_dir
            .parent()
            .context("bare git directory has no parent")?;
        return Ok(parent.join(ticket));
    }

    let main = main_worktree_root()?;
    let dir = main.join(".worktrees");
    fs::create_dir_all(&dir)?;
    ignore_worktrees(&main)?;
    Ok(dir.join(ticket))
}

fn main_worktree_root() -> Result<PathBuf> {
    let listing = git::run(&["worktree", "list", "--porcelain"])?;
    let first = listing
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))
        .context("cannot find the main worktree")?;
    Ok(PathBuf::from(first))
}

/// `.worktrees/` holds Amaca's worktrees, so the main repo must ignore it.
fn ignore_worktrees(root: &Path) -> Result<()> {
    let path = root.join(".gitignore");
    let content = fs::read_to_string(&path).unwrap_or_default();
    if content.lines().any(|line| line.trim() == ".worktrees/") {
        return Ok(());
    }

    let mut content = content;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(".worktrees/\n");
    fs::write(&path, content)?;
    Ok(())
}

fn dev_branch() -> Result<String> {
    if git::succeeds(&["config", "--get", "repo.devBranch"])? {
        git::run(&["config", "--get", "repo.devBranch"])
    } else {
        Ok("develop".to_owned())
    }
}

/// Bare clones keep fetched heads as local refs, so prefer the remote-tracking
/// ref and fall back to the local one.
fn dev_start() -> Result<String> {
    let dev = dev_branch()?;
    if ref_exists(&format!("refs/remotes/origin/{dev}"))? {
        Ok(format!("origin/{dev}"))
    } else if ref_exists(&local_ref(&dev))? {
        Ok(dev)
    } else {
        bail!("dev branch {dev:?} not found: neither origin/{dev} nor {dev} exists")
    }
}

fn ref_exists(reference: &str) -> Result<bool> {
    git::succeeds(&["show-ref", "--verify", "--quiet", reference])
}

fn local_ref(branch: &str) -> String {
    format!("refs/heads/{branch}")
}
