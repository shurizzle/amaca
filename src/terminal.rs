use std::fmt;
use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};
use libc::termios;

/// Tells the terminal that the current directory changed (OSC 7).
pub fn set_cwd(path: &Path) -> Result<()> {
    let path = path.to_str().context("path is not valid UTF-8")?;
    let mut stdout = io::stdout().lock();
    write!(stdout, "\x1b]7;file://{}\x07", PercentEncode(path))?;
    stdout.flush()?;
    Ok(())
}

/// Percent-encodes a [`Display`], keeping unreserved characters and `/` as they
/// are, as the OSC 7 URL requires. Writes straight through the formatter, so it
/// allocates nothing.
pub struct PercentEncode<T>(pub T);

impl<T: fmt::Display> fmt::Display for PercentEncode<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(Encoder { out: f }, "{}", self.0)
    }
}

struct Encoder<'a, 'b> {
    out: &'a mut fmt::Formatter<'b>,
}

impl fmt::Write for Encoder<'_, '_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        for byte in text.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                    self.out.write_char(byte as char)?;
                }
                _ => {
                    self.out.write_char('%')?;
                    self.out.write_char(HEX[usize::from(byte >> 4)] as char)?;
                    self.out.write_char(HEX[usize::from(byte & 0x0F)] as char)?;
                }
            }
        }
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::PercentEncode;

    fn encoded(input: &str) -> String {
        PercentEncode(input).to_string()
    }

    #[test]
    fn keeps_unreserved_and_slashes() {
        assert_eq!(encoded("/tmp/ABC-1_x.y~z"), "/tmp/ABC-1_x.y~z");
    }

    #[test]
    fn encodes_spaces_and_non_ascii() {
        assert_eq!(encoded("/tmp/a b/ü"), "/tmp/a%20b/%C3%BC");
    }
}
