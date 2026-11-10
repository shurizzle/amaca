use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use super::Mux;
use crate::ticket::Ticket;

/// Returns the current multiplexer when Amaca runs inside a working kitty.
pub fn detect() -> Option<Mux> {
    std::env::var("KITTY_WINDOW_ID").ok()?;
    let status = Command::new("kitty")
        .args(["@", "ls"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        eprintln!(
            "warning: kitty found but its remote control is off: set allow_remote_control yes"
        );
        return None;
    }
    Some(Mux::Kitty)
}

/// Docks a 2-row banner window to the top edge of the current one, running the
/// hidden `ꙮ` command on the ticket, without taking the focus.
pub fn layout(ticket: &Ticket, cwd: &Path) -> Result<()> {
    let exe = std::env::current_exe().context("cannot resolve the amaca executable")?;

    // A dock keeps its size in cells and can never take the focus, so the
    // banner stays above the current window whatever the layout is.
    let status = Command::new("kitty")
        .args([
            "@",
            "launch",
            "--type=window-dock",
            "--dock-edge=top",
            "--dock-size=2",
            "--dock-skip-focus",
            "--no-response",
            "--cwd",
        ])
        .arg(cwd)
        .arg("--")
        .arg(&exe)
        .arg("ꙮ")
        .args([&ticket.id, &ticket.title, &ticket.summary])
        .stdout(Stdio::null())
        .status()
        .context("cannot run kitty @ launch")?;
    if !status.success() {
        bail!("kitty @ launch exited with {status}");
    }
    Ok(())
}
