use std::ffi::{CStr, CString, OsString};
use std::io;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as _;

/// A configured command: a shell line, or an argv run without a shell.
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[cfg_attr(test, schemars(untagged))]
#[derive(Debug)]
pub enum Run {
    /// Shell line, run through system(3). It cannot hold a NUL.
    Shell(#[cfg_attr(test, schemars(with = "String", extend("pattern" = r"^[^\x00]*$")))] CString),
    /// Argv run directly, without a shell. The first element is the program.
    Argv(#[cfg_attr(test, schemars(with = "Vec<String>", extend("minItems" = 1)))] Vec<OsString>),
}

impl<'de> Deserialize<'de> for Run {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Shell(String),
            Argv(Vec<String>),
        }

        match Raw::deserialize(deserializer)? {
            Raw::Shell(line) => CString::new(line)
                .map(Run::Shell)
                .map_err(serde::de::Error::custom),
            Raw::Argv(args) if args.is_empty() => Err(D::Error::custom("command array is empty")),
            Raw::Argv(args) => Ok(Run::Argv(args.into_iter().map(OsString::from).collect())),
        }
    }
}

/// Runs a configured command, inheriting the terminal.
pub fn run(command: &Run) -> Result<()> {
    match command {
        Run::Shell(line) => shell(line),
        Run::Argv(args) => argv(args),
    }
}

fn shell(command: &CStr) -> Result<()> {
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

fn argv(args: &[OsString]) -> Result<()> {
    let (program, rest) = args.split_first().context("command array is empty")?;
    let status = std::process::Command::new(program)
        .args(rest)
        .status()
        .with_context(|| format!("cannot run {program:?}"))?;
    if !status.success() {
        bail!("command {program:?} failed: {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Run, run};

    #[test]
    fn shell_succeeds_on_zero_exit() {
        run(&Run::Shell(c"true".to_owned())).expect("true exits with 0");
    }

    #[test]
    fn shell_fails_on_non_zero_exit() {
        assert!(run(&Run::Shell(c"exit 3".to_owned())).is_err());
    }

    #[test]
    fn argv_succeeds_on_zero_exit() {
        run(&Run::Argv(vec!["true".into()])).expect("true exits with 0");
    }

    #[test]
    fn argv_fails_on_non_zero_exit() {
        assert!(run(&Run::Argv(vec!["false".into()])).is_err());
    }
}
