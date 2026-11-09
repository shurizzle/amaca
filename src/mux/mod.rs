//! Terminal multiplexer backends: standalone multiplexers such as tmux,
//! screen and zellij, and terminals with the feature built in such as WezTerm.
//! Amaca runs inside one of them and lays the ticket out on the current pane.

pub mod screen;
pub mod tmux;
pub mod wezterm;
pub mod zellij;

use std::path::Path;

use anyhow::{Context, Result};

use crate::ticket::Ticket;

/// The multiplexer Amaca runs inside, with the id of the current pane.
pub enum Mux {
    Screen,
    Tmux(String),
    Wezterm(String),
    Zellij(String),
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
    /// Detects the multiplexer by trying the backends in priority order. The
    /// standalone multiplexers win over WezTerm: a terminal with a built-in
    /// multiplexer can also run one of them inside it. zellij wins over tmux
    /// because it commonly runs inside WezTerm, where it owns the panes.
    pub fn detect() -> Result<Self> {
        fallback!(
            zellij::detect,
            tmux::detect,
            screen::detect,
            wezterm::detect
        )
        .context("not inside zellij, tmux, screen or WezTerm: amaca needs a terminal multiplexer")
    }

    /// Splits the current pane, runs the `ꙮ` banner in the new pane, and
    /// gives the focus back to this pane.
    pub fn layout(&self, ticket: &Ticket, cwd: &Path) -> Result<()> {
        match self {
            Mux::Screen => screen::layout(ticket, cwd),
            Mux::Tmux(pane) => tmux::layout(ticket, cwd, pane),
            Mux::Wezterm(pane) => wezterm::layout(ticket, cwd, pane),
            Mux::Zellij(pane) => zellij::layout(ticket, cwd, pane),
        }
    }
}
