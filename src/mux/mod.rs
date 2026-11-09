//! Terminal multiplexer backends: standalone multiplexers such as tmux, and
//! terminals with the feature built in such as WezTerm. Amaca runs inside one
//! of them and lays the ticket out on the current pane.

pub mod tmux;
pub mod wezterm;

use std::path::Path;

use anyhow::{Context, Result};

use crate::ticket::Ticket;

/// The multiplexer Amaca runs inside, with the id of the current pane.
pub enum Mux {
    Tmux(String),
    Wezterm(String),
}

macro_rules! fallback {
    () => {
        (core::option::Option::<Mux>::None)
    };
    ($first:expr) => {
        $first()
    };
    ($first:expr, $($tries:expr),+ $(,)?) => {
        fallback!($first)$(.or_else($tries))+
    };
}

impl Mux {
    /// Detects the multiplexer by trying the backends in priority order. tmux
    /// wins over WezTerm: a terminal with a built-in multiplexer can also run
    /// tmux inside it.
    pub fn detect() -> Result<Self> {
        fallback!(tmux::detect, wezterm::detect)
            .context("not inside tmux or WezTerm: amaca needs a terminal multiplexer")
    }

    /// Splits the current pane, runs the `ꙮ` banner on top, and gives the
    /// focus back to this pane.
    pub fn layout(&self, ticket: &Ticket, cwd: &Path) -> Result<()> {
        match self {
            Mux::Tmux(pane) => tmux::layout(ticket, cwd, pane),
            Mux::Wezterm(pane) => wezterm::layout(ticket, cwd, pane),
        }
    }
}
