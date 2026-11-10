// The ꙮ subcommand name is intentionally exotic.
#![allow(uncommon_codepoints)]

mod claude;
mod command;
mod config;
mod git;
mod mux;
mod review;
mod session;
mod terminal;
mod ticket;
mod worktree;

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use mux::Mux;
use ticket::Source;

#[derive(Parser)]
#[command(version, about, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Jira link or key; defaults to the current git branch.
    input: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Set up the worktree for a ticket and start Claude on it.
    Solve {
        /// Jira link or key; defaults to the current git branch.
        input: Option<String>,
    },
    /// Review a GitHub pull request in a throwaway worktree.
    Review {
        /// GitHub pull request link or number.
        input: String,
    },
    /// Print the arguments on the alternate screen and wait forever.
    #[command(hide = true)]
    ꙮ {
        first: String,
        second: String,
        third: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match match cli.command {
        Some(Command::Solve { input }) => solve(input),
        Some(Command::Review { input }) => review(&input),
        Some(Command::ꙮ {
            first,
            second,
            third,
        }) => hold(&first, &second, &third),
        None => solve(cli.input),
    } {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn hold(first: &str, second: &str, third: &str) -> Result<()> {
    let screen = terminal::Screen::enter()?;
    screen.set_title(first)?;
    let mut stdout = io::stdout().lock();
    write!(stdout, "{first} - {second}\n{third}")?;
    stdout.flush()?;
    drop(stdout);
    loop {
        std::thread::park();
    }
}

fn solve(input: Option<String>) -> Result<()> {
    git::require_repository()?;
    let mux = Mux::detect()?;
    let config = config::Config::load()?;
    let (input, source) = match input {
        Some(input) => (input, Source::Argument),
        None => (git::current_branch()?, Source::Branch),
    };
    let session_id = session::new_id();
    let ticket = ticket::fetch(&input, source, &session_id)?;
    let worktree = worktree::ensure(&ticket.id)?;
    std::env::set_current_dir(&worktree.path)
        .with_context(|| format!("cannot enter {}", worktree.path.display()))?;
    terminal::set_cwd(&worktree.path)?;

    if worktree.created
        && let Some(setup) = &config.command.created
    {
        command::run(setup)?;
    }

    mux.layout(&ticket, &worktree.path)?;
    let prompt = config.prompt(worktree.created, &ticket.id);
    claude::launch(&session_id, &ticket, &prompt)
}

fn review(input: &str) -> Result<()> {
    git::require_repository()?;
    let mux = Mux::detect()?;
    let config = config::Config::load()?;
    let branch = review::branch(input)?;
    let key = review::ticket_key(&branch)
        .with_context(|| format!("branch {branch:?} has no Jira ticket key"))?;
    let session_id = session::new_id();
    let ticket = ticket::fetch(&key, Source::Branch, &session_id)?;
    let worktree = worktree::review(&ticket.id, &branch)?;
    std::env::set_current_dir(&worktree.path)
        .with_context(|| format!("cannot enter {}", worktree.path.display()))?;
    terminal::set_cwd(&worktree.path)?;

    if let Some(setup) = &config.command.created {
        command::run(setup)?;
    }

    review::load_conversation(input, &session_id)?;
    mux.layout(&ticket, &worktree.path)?;
    let prompt = config.review_prompt(&ticket.id);
    claude::launch(&session_id, &ticket, &prompt)
}
