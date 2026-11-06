use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::ticket::Ticket;

/// Resumes the ticket session interactively in the current terminal, named
/// after the ticket, asking Claude to solve the ticket.
pub fn launch(session_id: &str, ticket: &Ticket, created: bool) -> Result<()> {
    let setup = if created {
        "The worktree was just created: do any setup it needs. "
    } else {
        ""
    };
    let prompt = format!(
        "{setup}Solve the ticket {}. The ticket is already in this session's context. \
         Follow the repository conventions.",
        ticket.id
    );
    let status = Command::new("claude")
        .args(["--resume", session_id, "--name", &ticket.id, &prompt])
        .status()
        .context("cannot run claude")?;
    if !status.success() {
        bail!("claude exited with {status}");
    }
    Ok(())
}
