use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::ticket::Ticket;

/// Returns the current pane id, failing early when Amaca runs outside WezTerm
/// or the mux does not answer.
pub fn pane() -> Result<String> {
    let pane = std::env::var("WEZTERM_PANE")
        .context("WEZTERM_PANE is not set; run Amaca inside WezTerm")?;
    let status = Command::new("wezterm")
        .args(["cli", "list"])
        .stdout(Stdio::null())
        .status()
        .context("cannot run wezterm cli list")?;
    if !status.success() {
        bail!("wezterm cli list exited with {status}; is the WezTerm mux reachable?");
    }
    Ok(pane)
}

/// Splits the current pane horizontally with a 2-row pane above it, running
/// the hidden `ꙮ` command on the ticket, then returns the focus to this pane.
pub fn layout(ticket: &Ticket, cwd: &Path, pane: &str) -> Result<()> {
    let exe = std::env::current_exe().context("cannot resolve the amaca executable")?;

    let status = Command::new("wezterm")
        .args(["cli", "split-pane", "--top", "--cells", "2", "--cwd"])
        .arg(cwd)
        .arg("--")
        .arg(&exe)
        .arg("ꙮ")
        .args([&ticket.id, &ticket.title, &ticket.summary])
        // split-pane prints the new pane id; keep it off the terminal.
        .stdout(Stdio::null())
        .status()
        .context("cannot run wezterm cli split-pane")?;
    if !status.success() {
        bail!("wezterm cli split-pane exited with {status}");
    }

    let status = Command::new("wezterm")
        .args(["cli", "activate-pane", "--pane-id", pane])
        .status()
        .context("cannot run wezterm cli activate-pane")?;
    if !status.success() {
        bail!("wezterm cli activate-pane exited with {status}");
    }
    Ok(())
}
