use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use uuid::Uuid;

use crate::git;

pub struct Worktree {
    pub path: PathBuf,
    pub created: bool,
}

/// A throwaway worktree on a pull request branch. It is removed when dropped,
/// after Claude exits, together with its local branch.
pub struct Review {
    pub path: PathBuf,
    name: String,
    original: PathBuf,
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

/// Creates a throwaway worktree on the pull request branch, fetched from
/// origin. The local branch is `review-<ticket>`, with a random suffix when
/// that name is taken. Dropping the result removes the worktree.
pub fn review(ticket: &str, branch: &str) -> Result<Review> {
    git::run(&["fetch", "origin", branch])?;
    let start = if ref_exists(&format!("refs/remotes/origin/{branch}"))? {
        format!("origin/{branch}")
    } else if ref_exists(&local_ref(branch))? {
        branch.to_owned()
    } else {
        bail!("branch {branch:?} not found: neither origin/{branch} nor {branch} exists")
    };

    let name = review_name(ticket)?;
    let path = target_path(&name)?;
    let path_str = path.to_str().context("worktree path is not valid UTF-8")?;
    git::run(&["worktree", "add", "-b", &name, path_str, &start])?;

    let original = std::env::current_dir().context("cannot read the current directory")?;
    Ok(Review {
        path,
        name,
        original,
    })
}

fn review_name(ticket: &str) -> Result<String> {
    let base = format!("review-{ticket}");
    let mut name = base.clone();
    loop {
        let path = target_path(&name)?;
        if !ref_exists(&local_ref(&name))? && !path.exists() {
            return Ok(name);
        }
        name = format!("{base}-{}", &Uuid::new_v4().simple().to_string()[..8]);
    }
}

impl Drop for Review {
    fn drop(&mut self) {
        // git refuses to remove the worktree Amaca is inside.
        let _ = std::env::set_current_dir(&self.original);
        let _ = crate::terminal::set_cwd(&self.original);
        let Some(path) = self.path.to_str() else {
            return;
        };
        if let Err(err) = git::run(&["worktree", "remove", "--force", path]) {
            eprintln!("warning: cannot remove the review worktree: {err:#}");
            // The worktree still holds the branch, so deleting it would fail.
            return;
        }
        if let Err(err) = git::run(&["branch", "-D", &self.name]) {
            eprintln!("warning: cannot delete the review branch: {err:#}");
        }
    }
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
    // A linked worktree of a bare repository is not itself bare, so ask git
    // about the shared directory instead of the current one.
    let git_dir = PathBuf::from(git::run(&[
        "rev-parse",
        "--path-format=absolute",
        "--git-common-dir",
    ])?);
    let git_dir_arg = format!(
        "--git-dir={}",
        git_dir
            .to_str()
            .context("git directory path is not valid UTF-8")?
    );
    if git::run(&[&git_dir_arg, "rev-parse", "--is-bare-repository"])? == "true" {
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
