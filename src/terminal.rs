use std::fmt;
use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};

/// Tells the terminal that the current directory changed (OSC 7).
pub fn set_cwd(path: &Path) -> Result<()> {
    let path = path.to_str().context("path is not valid UTF-8")?;
    let path = if cfg!(windows) {
        windows_url_path(path)
    } else {
        path.to_owned()
    };
    let mut stdout = io::stdout().lock();
    write!(stdout, "\x1b]7;file://{}\x07", PercentEncode(path))?;
    stdout.flush()?;
    Ok(())
}

/// The path part of a `file:` URL for a Windows path: forward slashes, a
/// leading `/` before the drive letter, no `\\?\` verbatim prefix.
/// `C:\Users\me` becomes `/C:/Users/me`.
fn windows_url_path(path: &str) -> String {
    let path = path
        .strip_prefix(r"\\?\")
        .unwrap_or(path)
        .replace('\\', "/");
    if path.as_bytes().get(1) == Some(&b':') {
        format!("/{path}")
    } else {
        path
    }
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
    original: raw::Original,
}

impl Screen {
    pub fn enter() -> Result<Self> {
        let screen = Self {
            original: raw::enter()?,
        };
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
        raw::restore(&self.original);
    }
}

/// Terminal input mode: no echo, no line buffering.
#[cfg(unix)]
mod raw {
    use std::io;

    use anyhow::{Context, Result};
    use libc::termios;

    pub type Original = termios;

    pub fn enter() -> Result<Original> {
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
        Ok(original)
    }

    pub fn restore(original: &Original) {
        // SAFETY: original came from tcgetattr on the same file descriptor.
        unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, original) };
    }
}

/// Console modes: no echo and no line input on stdin, ANSI escapes on stdout.
#[cfg(windows)]
mod raw {
    use std::io;

    use anyhow::{Context, Result};
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::Console::{
        ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode,
        GetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetConsoleMode,
    };

    pub struct Original {
        input: u32,
        output: u32,
    }

    fn mode(handle: HANDLE) -> Result<u32> {
        let mut mode = 0;
        // SAFETY: handle comes from GetStdHandle; mode is a valid out pointer.
        if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
            return Err(io::Error::last_os_error()).context("cannot read console settings");
        }
        Ok(mode)
    }

    fn set_mode(handle: HANDLE, mode: u32) -> Result<()> {
        // SAFETY: handle comes from GetStdHandle.
        if unsafe { SetConsoleMode(handle, mode) } == 0 {
            return Err(io::Error::last_os_error()).context("cannot set console settings");
        }
        Ok(())
    }

    pub fn enter() -> Result<Original> {
        // SAFETY: GetStdHandle has no preconditions.
        let (input, output) = unsafe {
            (
                GetStdHandle(STD_INPUT_HANDLE),
                GetStdHandle(STD_OUTPUT_HANDLE),
            )
        };
        let original = Original {
            input: mode(input)?,
            output: mode(output)?,
        };
        set_mode(
            input,
            original.input & !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT),
        )?;
        set_mode(output, original.output | ENABLE_VIRTUAL_TERMINAL_PROCESSING)?;
        Ok(original)
    }

    pub fn restore(original: &Original) {
        // SAFETY: GetStdHandle has no preconditions.
        let (input, output) = unsafe {
            (
                GetStdHandle(STD_INPUT_HANDLE),
                GetStdHandle(STD_OUTPUT_HANDLE),
            )
        };
        let _ = set_mode(input, original.input);
        let _ = set_mode(output, original.output);
    }
}

#[cfg(test)]
mod tests {
    use super::{PercentEncode, windows_url_path};

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

    #[test]
    fn converts_windows_paths_to_url_paths() {
        assert_eq!(windows_url_path(r"C:\Users\me\repo"), "/C:/Users/me/repo");
        assert_eq!(windows_url_path(r"\\?\C:\repo"), "/C:/repo");
    }
}
