use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use super::Mux;
use crate::ticket::Ticket;

/// Returns the current pane when Amaca runs inside a working zellij.
pub fn detect() -> Option<Mux> {
    let pane = std::env::var("ZELLIJ_PANE_ID").ok()?;
    let status = Command::new("zellij")
        .args(["action", "list-panes"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    status.success().then_some(Mux::Zellij(pane))
}

/// Splits the current pane with a small pane above it, running the hidden `ꙮ`
/// command on the ticket, then returns the focus to this pane.
pub fn layout(ticket: &Ticket, cwd: &Path, pane: &str) -> Result<()> {
    let exe = std::env::current_exe().context("cannot resolve the amaca executable")?;

    // `new-pane` only splits right or down: open below, then move the pane up.
    // `new-pane` prints the new pane id, so its output cannot go to the
    // terminal.
    let output = Command::new("zellij")
        .args(["action", "new-pane", "--direction", "down", "--cwd"])
        .arg(cwd)
        .arg("--")
        .arg(&exe)
        .arg("ꙮ")
        .args([&ticket.id, &ticket.title, &ticket.summary])
        .output()
        .context("cannot run zellij action new-pane")?;
    if !output.status.success() {
        bail!("zellij action new-pane exited with {}", output.status);
    }
    let created = String::from_utf8(output.stdout)
        .context("zellij action new-pane returned invalid UTF-8")?
        .trim()
        .to_owned();

    move_up(&created)?;
    shrink(&created)?;

    let status = Command::new("zellij")
        .args(["action", "focus-pane-id", pane])
        .status()
        .context("cannot run zellij action focus-pane-id")?;
    if !status.success() {
        bail!("zellij action focus-pane-id exited with {status}");
    }
    Ok(())
}

/// Puts the new pane above the current one: zellij cannot open a pane upwards,
/// so moving it up swaps the two panes between the top and bottom slots.
fn move_up(pane: &str) -> Result<()> {
    let status = Command::new("zellij")
        .args(["action", "move-pane", "--pane-id", pane, "up"])
        .stdout(Stdio::null())
        .status()
        .context("cannot run zellij action move-pane")?;
    if !status.success() {
        bail!("zellij action move-pane exited with {status}");
    }
    Ok(())
}

/// `new-pane` has no size option: it opens at half height. Shrink the pane one
/// step at a time until zellij keeps it at its minimum height (6 rows with the
/// frame).
///
/// shortcut: one `zellij action resize` call per step, replace with a single
/// call when `new-pane` gains a size option.
fn shrink(pane: &str) -> Result<()> {
    let mut previous = u64::MAX;
    for _ in 0..100 {
        let current = rows(pane)?;
        if current >= previous {
            return Ok(());
        }
        previous = current;
        let status = Command::new("zellij")
            .args(["action", "resize", "--pane-id", pane, "decrease", "down"])
            .stdout(Stdio::null())
            .status()
            .context("cannot run zellij action resize")?;
        if !status.success() {
            bail!("zellij action resize exited with {status}");
        }
    }
    bail!("zellij pane {pane} did not reach its minimum height")
}

/// Reads the height of a pane from `zellij action list-panes --json`.
fn rows(pane: &str) -> Result<u64> {
    let number = pane
        .rsplit('_')
        .next()
        .and_then(|number| number.parse::<u64>().ok())
        .with_context(|| format!("unexpected zellij pane id {pane}"))?;

    let output = Command::new("zellij")
        .args(["action", "list-panes", "--json", "--geometry"])
        .output()
        .context("cannot run zellij action list-panes")?;
    if !output.status.success() {
        bail!("zellij action list-panes exited with {}", output.status);
    }
    let panes: Vec<serde_json::Value> =
        serde_json::from_slice(&output.stdout).context("cannot parse zellij panes")?;
    panes
        .iter()
        .find(|entry| entry["is_plugin"] == false && entry["id"].as_u64() == Some(number))
        .and_then(|entry| entry["pane_rows"].as_u64())
        .with_context(|| format!("cannot find zellij pane {pane}"))
}
