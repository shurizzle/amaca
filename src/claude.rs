use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::ticket::Ticket;

/// Resumes the ticket session interactively in the current terminal, named
/// after the ticket, asking Claude to solve the ticket with `prompt`.
pub fn launch(session_id: &str, ticket: &Ticket, prompt: &str) -> Result<()> {
    let status = Command::new("claude")
        .args(["--resume", session_id, "--name", &ticket.id, prompt])
        .status()
        .context("cannot run claude")?;
    if !status.success() {
        bail!("claude exited with {status}");
    }
    Ok(())
}
