use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};
use libc::termios;

/// Tells the terminal that the current directory changed (OSC 7).
pub fn set_cwd(path: &Path) -> Result<()> {
    let path = path.to_str().context("path is not valid UTF-8")?;
    let mut stdout = io::stdout().lock();
    // shortcut: the path is not percent-encoded; ticket worktrees avoid spaces.
    stdout.write_all(b"\x1b]7;file://")?;
    stdout.write_all(path.as_bytes())?;
    stdout.write_all(b"\x07")?;
    stdout.flush()?;
    Ok(())
}

/// Puts the terminal in the alternate screen without echo or line buffering,
/// and restores it on drop.
pub struct Screen {
    original: termios,
}

impl Screen {
    pub fn enter() -> Result<Self> {
        // SAFETY: stdin is a valid file descriptor; tcgetattr fills the struct
        // on success.
        let mut original = unsafe { std::mem::zeroed::<termios>() };
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut original) } != 0 {
            return Err(io::Error::last_os_error()).context("cannot read terminal settings");
        }

        let mut raw = original;
        raw.c_lflag &= !(libc::ECHO | libc::ICANON);
        // SAFETY: raw is a valid termios for the same file descriptor.
        if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw) } != 0 {
            return Err(io::Error::last_os_error()).context("cannot set terminal settings");
        }

        let screen = Self { original };
        screen.escape(b"\x1b[?1049h\x1b[?25l")?;
        Ok(screen)
    }

    pub fn set_title(&self, title: &str) -> Result<()> {
        let mut stdout = io::stdout().lock();
        stdout.write_all(b"\x1b]0;")?;
        stdout.write_all(title.as_bytes())?;
        stdout.write_all(b"\x07")?;
        stdout.flush()?;
        Ok(())
    }

    fn escape(&self, bytes: &[u8]) -> Result<()> {
        let mut stdout = io::stdout();
        stdout.write_all(bytes)?;
        stdout.flush()?;
        Ok(())
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = self.escape(b"\x1b[?25h\x1b[?1049l");
        // SAFETY: original came from tcgetattr on the same file descriptor.
        unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.original) };
    }
}
