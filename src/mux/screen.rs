use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use super::Mux;
use crate::ticket::Ticket;

/// Rows given to the banner region: three content rows plus the region caption
/// screen adds to split displays.
const BANNER_ROWS: &str = "4";

/// Returns the current multiplexer when Amaca runs inside a working screen.
pub fn detect() -> Option<Mux> {
    std::env::var("STY").ok()?;
    let status = Command::new("screen")
        .args(["-Q", "number"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    status.success().then_some(Mux::Screen)
}

/// Splits the current region with a small region below it, runs the hidden `ꙮ`
/// command on the ticket there, then gives the focus back to this region.
pub fn layout(ticket: &Ticket, cwd: &Path) -> Result<()> {
    let exe = std::env::current_exe().context("cannot resolve the amaca executable")?;

    // The new region opens below and the focus stays put: move into it.
    run(&["-X", "split"])?;
    run(&["-X", "focus", "down"])?;
    run(&["-X", "resize", BANNER_ROWS])?;

    // The new region shows the internal blank window, which runs nothing:
    // create a window with the banner and select it in this region.
    run(&[
        "-X",
        "screen",
        "-t",
        &ticket.id,
        "sh",
        "-c",
        &banner_command(&exe, ticket, cwd),
    ])?;

    // Back to the region above, where Claude launches.
    run(&["-X", "focus", "up"])
}

/// Builds the shell line that enters the worktree and runs the hidden `ꙮ`
/// command: screen has no cwd option for a new window.
fn banner_command(exe: &Path, ticket: &Ticket, cwd: &Path) -> String {
    format!(
        "cd {} && exec {} ꙮ {} {} {}",
        quote(&cwd.to_string_lossy()),
        quote(&exe.to_string_lossy()),
        quote(&ticket.id),
        quote(&ticket.title),
        quote(&ticket.summary),
    )
}

/// Single-quotes `value` for `sh -c`.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Sends one command to the screen session Amaca runs in.
fn run(args: &[&str]) -> Result<()> {
    let status = Command::new("screen")
        .args(args)
        .status()
        .context("cannot run screen")?;
    if !status.success() {
        bail!("screen {} exited with {status}", args.join(" "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::quote;

    #[test]
    fn quote_wraps_and_escapes_single_quotes() {
        assert_eq!(quote("plain"), "'plain'");
        assert_eq!(quote("with space"), "'with space'");
        assert_eq!(quote("it's"), r"'it'\''s'");
        assert_eq!(quote("two\nlines"), "'two\nlines'");
    }
}
