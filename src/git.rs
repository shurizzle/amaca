use std::process::Command;

use anyhow::{Context, Result, bail};

/// Fails early when Amaca is not inside a git repository, bare included.
pub fn require_repository() -> Result<()> {
    if !succeeds(&["rev-parse", "--git-dir"])? {
        bail!("not a git repository; pass a ticket link or key as argument");
    }
    Ok(())
}

pub fn current_branch() -> Result<String> {
    if !succeeds(&["rev-parse", "--is-inside-work-tree"])? {
        bail!("not a git work tree; pass a ticket link or key as argument");
    }

    // symbolic-ref works on an unborn branch (no commits yet); rev-parse does not.
    run(&["symbolic-ref", "--short", "HEAD"])
        .context("detached HEAD; pass a ticket link or key as argument")
}

/// Runs git and returns true when it exits successfully. Failures are answers
/// here (missing branch, unset config), not errors.
pub fn succeeds(args: &[&str]) -> Result<bool> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("cannot run git")?;
    Ok(output.status.success())
}

pub fn run(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("cannot run git")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
