//! Terminal multiplexer backends: standalone multiplexers such as tmux,
//! screen and zellij, and terminals with the feature built in such as WezTerm
//! and iTerm2. Amaca runs inside one of them and lays the ticket out on the
//! current pane.

#[cfg(target_os = "macos")]
pub mod iterm2;
#[cfg(unix)]
pub mod kitty;
#[cfg(unix)]
pub mod screen;
#[cfg(unix)]
pub mod tmux;
pub mod wezterm;
#[cfg(unix)]
pub mod zellij;

use std::path::Path;

use anyhow::{Context, Result};

use crate::ticket::Ticket;

/// The multiplexer Amaca runs inside, with the id of the current pane.
pub enum Mux {
    #[cfg(target_os = "macos")]
    Iterm2(String),
    #[cfg(unix)]
    Kitty,
    #[cfg(unix)]
    Screen,
    #[cfg(unix)]
    Tmux(String),
    Wezterm(String),
    #[cfg(unix)]
    Zellij(String),
}

impl Mux {
    /// Detects the multiplexer by trying the backends in priority order. The
    /// standalone multiplexers win over the terminals with a built-in
    /// multiplexer: a terminal with a built-in multiplexer can also run one of
    /// them inside it. zellij wins over tmux because it commonly runs inside
    /// WezTerm or iTerm2, where it owns the panes. kitty, iTerm2 and WezTerm
    /// cannot be the outer terminal at the same time, so their order does not
    /// matter.
    pub fn detect() -> Result<Self> {
        // zellij, tmux, screen and kitty do not run natively on Windows, and
        // the iTerm2 backend is macOS-only: a backend missing on the platform
        // is skipped.
        let detectors: &[fn() -> Option<Mux>] = &[
            #[cfg(unix)]
            zellij::detect,
            #[cfg(unix)]
            tmux::detect,
            #[cfg(unix)]
            screen::detect,
            #[cfg(unix)]
            kitty::detect,
            #[cfg(target_os = "macos")]
            iterm2::detect,
            wezterm::detect,
        ];
        let detected = detectors.iter().find_map(|detect| detect());
        detected.context(
            "not inside zellij, tmux, screen, kitty, iTerm2 or WezTerm: amaca needs a terminal \
             multiplexer",
        )
    }

    /// Splits the current pane, runs the `ꙮ` banner in the new pane, and
    /// gives the focus back to this pane.
    pub fn layout(&self, ticket: &Ticket, cwd: &Path) -> Result<()> {
        match self {
            #[cfg(target_os = "macos")]
            Mux::Iterm2(session) => iterm2::layout(ticket, cwd, session),
            #[cfg(unix)]
            Mux::Kitty => kitty::layout(ticket, cwd),
            #[cfg(unix)]
            Mux::Screen => screen::layout(ticket, cwd),
            #[cfg(unix)]
            Mux::Tmux(pane) => tmux::layout(ticket, cwd, pane),
            Mux::Wezterm(pane) => wezterm::layout(ticket, cwd, pane),
            #[cfg(unix)]
            Mux::Zellij(pane) => zellij::layout(ticket, cwd, pane),
        }
    }
}
