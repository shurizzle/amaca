//! Terminal multiplexer backends: standalone multiplexers such as tmux,
//! screen and zellij, and terminals with the feature built in such as WezTerm
//! and iTerm2. Amaca runs inside one of them and lays the ticket out on the
//! current pane.

#[cfg(target_os = "macos")]
pub mod iterm2;
pub mod screen;
pub mod tmux;
pub mod wezterm;
pub mod zellij;

use std::path::Path;

use anyhow::{Context, Result};

use crate::ticket::Ticket;

/// The multiplexer Amaca runs inside, with the id of the current pane.
pub enum Mux {
    #[cfg(target_os = "macos")]
    Iterm2(String),
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
    /// standalone multiplexers win over the terminals with a built-in
    /// multiplexer: a terminal with a built-in multiplexer can also run one of
    /// them inside it. zellij wins over tmux because it commonly runs inside
    /// WezTerm or iTerm2, where it owns the panes.
    pub fn detect() -> Result<Self> {
        #[cfg(target_os = "macos")]
        let detected = fallback!(
            zellij::detect,
            tmux::detect,
            screen::detect,
            iterm2::detect,
            wezterm::detect
        );
        // The iTerm2 backend is macOS-only; without it the chain skips it.
        #[cfg(not(target_os = "macos"))]
        let detected = fallback!(
            zellij::detect,
            tmux::detect,
            screen::detect,
            wezterm::detect
        );
        detected.context(
            "not inside zellij, tmux, screen, iTerm2 or WezTerm: amaca needs a terminal \
             multiplexer",
        )
    }

    /// Splits the current pane, runs the `ꙮ` banner in the new pane, and
    /// gives the focus back to this pane.
    pub fn layout(&self, ticket: &Ticket, cwd: &Path) -> Result<()> {
        match self {
            #[cfg(target_os = "macos")]
            Mux::Iterm2(session) => iterm2::layout(ticket, cwd, session),
            Mux::Screen => screen::layout(ticket, cwd),
            Mux::Tmux(pane) => tmux::layout(ticket, cwd, pane),
            Mux::Wezterm(pane) => wezterm::layout(ticket, cwd, pane),
            Mux::Zellij(pane) => zellij::layout(ticket, cwd, pane),
        }
    }
}
