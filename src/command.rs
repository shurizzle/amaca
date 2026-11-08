use std::ffi::CStr;
use std::io;

use anyhow::{Context, Result, bail};

/// Runs a shell command through `system(3)`, inheriting the terminal.
pub fn run(command: &CStr) -> Result<()> {
    // SAFETY: command is a valid, NUL-terminated C string that outlives the call.
    let status = unsafe { libc::system(command.as_ptr()) };
    if status == -1 {
        return Err(io::Error::last_os_error()).context("cannot run command");
    }
    if status != 0 {
        bail!("command {command:?} failed: {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn succeeds_on_zero_exit() {
        run(c"true").expect("true exits with 0");
    }

    #[test]
    fn fails_on_non_zero_exit() {
        assert!(run(c"exit 3").is_err());
    }
}
