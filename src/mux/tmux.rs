use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use super::Mux;
use crate::ticket::Ticket;

/// Returns the current pane when Amaca runs inside a working tmux.
pub fn detect() -> Option<Mux> {
    let pane = std::env::var("TMUX_PANE").ok()?;
    let status = Command::new("tmux")
        .args(["list-panes"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    status.success().then_some(Mux::Tmux(pane))
}

/// Splits the current pane vertically with a 2-row pane above it, running the
/// hidden `ꙮ` command on the ticket, then returns the focus to this pane.
pub fn layout(ticket: &Ticket, cwd: &Path, pane: &str) -> Result<()> {
    let exe = std::env::current_exe().context("cannot resolve the amaca executable")?;

    let status = Command::new("tmux")
        .args(["split-window", "-v", "-b", "-l", "2", "-c"])
        .arg(cwd)
        .args(["-t", pane])
        .arg(&exe)
        .arg("ꙮ")
        .args([&ticket.id, &ticket.title, &ticket.summary])
        .stdout(Stdio::null())
        .status()
        .context("cannot run tmux split-window")?;
    if !status.success() {
        bail!("tmux split-window exited with {status}");
    }

    let status = Command::new("tmux")
        .args(["select-pane", "-t", pane])
        .status()
        .context("cannot run tmux select-pane")?;
    if !status.success() {
        bail!("tmux select-pane exited with {status}");
    }
    Ok(())
}
