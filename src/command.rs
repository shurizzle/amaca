use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use mlua::{Table, Value};

/// What to execute.
#[derive(Debug, PartialEq)]
pub enum Kind {
    /// Shell line, run through `sh -c`, or `cmd /C` on Windows.
    Shell(String),
    /// Argv run directly, without a shell. The first element is the program.
    Argv(Vec<OsString>),
}

/// A configured command, with optional working directory and extra
/// environment variables.
#[derive(Debug, PartialEq)]
pub struct Run {
    pub kind: Kind,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

impl Run {
    pub fn shell(line: impl Into<String>) -> Self {
        Self {
            kind: Kind::Shell(line.into()),
            cwd: None,
            env: Vec::new(),
        }
    }

    #[cfg(all(test, unix))]
    pub fn argv(args: Vec<OsString>) -> Self {
        Self {
            kind: Kind::Argv(args),
            cwd: None,
            env: Vec::new(),
        }
    }

    /// Reads a Lua value: a shell line, or a table holding either an argv
    /// (array part) or `shell`, plus optional `cwd` and `env`.
    pub fn from_lua(value: Value) -> Result<Self> {
        let table = match value {
            Value::String(line) => return Ok(Self::shell(line.to_str()?.to_owned())),
            Value::Table(table) => table,
            other => bail!(
                "command must be a string or a table, got {}",
                other.type_name()
            ),
        };
        let argv = table
            .sequence_values::<String>()
            .map(|arg| arg.map(OsString::from))
            .collect::<mlua::Result<Vec<_>>>()
            .context("command array must hold strings")?;
        let shell: Option<String> = table.get("shell")?;
        let kind = match (argv.is_empty(), shell) {
            (false, None) => Kind::Argv(argv),
            (true, Some(line)) => Kind::Shell(line),
            (true, None) => bail!("command table needs an argv array or `shell`"),
            (false, Some(_)) => bail!("command table cannot hold both an argv array and `shell`"),
        };
        let cwd: Option<String> = table.get("cwd")?;
        let env = match table.get::<Option<Table>>("env")? {
            Some(env) => env
                .pairs::<String, String>()
                .collect::<mlua::Result<_>>()
                .context("`env` must map strings to strings")?,
            None => Vec::new(),
        };
        for key in table.pairs::<Value, Value>() {
            if let (Value::String(key), _) = key? {
                let key = key.to_str()?;
                if !["shell", "cwd", "env"].contains(&&*key) {
                    bail!("unknown command key {key:?}");
                }
            }
        }
        Ok(Self {
            kind,
            cwd: cwd.map(PathBuf::from),
            env,
        })
    }
}

/// A command that runs `line` through the platform shell: `sh -c`, or
/// `cmd /C` on Windows.
#[cfg(not(windows))]
fn shell_command(line: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new("/bin/sh");
    cmd.arg("-c").arg(line);
    cmd
}

#[cfg(windows)]
fn shell_command(line: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;

    let mut cmd = std::process::Command::new("cmd");
    // raw_arg: cmd.exe does not parse its command line like the C runtime.
    cmd.arg("/C").raw_arg(line);
    cmd
}

/// Runs a configured command, inheriting the terminal.
pub fn run(command: &Run) -> Result<()> {
    let (mut cmd, name) = match &command.kind {
        Kind::Shell(line) => (shell_command(line), format!("{line:?}")),
        Kind::Argv(args) => {
            let (program, rest) = args.split_first().context("command array is empty")?;
            let mut cmd = std::process::Command::new(program);
            cmd.args(rest);
            (cmd, format!("{program:?}"))
        }
    };
    if let Some(cwd) = &command.cwd {
        cmd.current_dir(cwd);
    }
    cmd.envs(command.env.iter().map(|(k, v)| (k, v)));
    let status = cmd.status().with_context(|| format!("cannot run {name}"))?;
    if !status.success() {
        bail!("command {name} failed: {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use mlua::Lua;

    use super::{Kind, Run, run};

    fn parse(code: &str) -> anyhow::Result<Run> {
        let lua = Lua::new();
        Run::from_lua(lua.load(code).eval()?)
    }

    #[test]
    #[cfg(unix)]
    fn shell_succeeds_on_zero_exit() {
        run(&Run::shell("true")).expect("true exits with 0");
    }

    #[test]
    fn shell_fails_on_non_zero_exit() {
        assert!(run(&Run::shell("exit 3")).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn argv_succeeds_on_zero_exit() {
        run(&Run::argv(vec!["true".into()])).expect("true exits with 0");
    }

    #[test]
    #[cfg(unix)]
    fn argv_fails_on_non_zero_exit() {
        assert!(run(&Run::argv(vec!["false".into()])).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn applies_cwd_and_env() {
        let dir = std::env::temp_dir().canonicalize().expect("temp dir");
        let mut cmd = Run::shell(r#"test "$X" = y && test "$(pwd -P)" = "$D""#);
        cmd.cwd = Some(dir.clone());
        cmd.env = vec![
            ("X".into(), "y".into()),
            ("D".into(), dir.to_str().unwrap().into()),
        ];
        run(&cmd).expect("cwd and env applied");
    }

    #[test]
    fn parses_lua_forms() {
        assert_eq!(parse(r#""make""#).unwrap(), Run::shell("make"));
        let run = parse(r#"{ "make", "setup", cwd = "sub", env = { A = "b" } }"#).unwrap();
        assert_eq!(run.kind, Kind::Argv(vec!["make".into(), "setup".into()]));
        assert_eq!(run.cwd.as_deref(), Some(std::path::Path::new("sub")));
        assert_eq!(run.env, [("A".to_owned(), "b".to_owned())]);
        assert_eq!(
            parse(r#"{ shell = "a && b" }"#).unwrap(),
            Run::shell("a && b")
        );
    }

    #[test]
    fn rejects_bad_lua_forms() {
        assert!(parse("{}").is_err(), "empty table");
        assert!(parse(r#"{ "a", shell = "b" }"#).is_err(), "argv and shell");
        assert!(
            parse(r#"{ shell = "a", bogus = 1 }"#).is_err(),
            "unknown key"
        );
        assert!(parse("42").is_err(), "number");
        assert!(
            parse(r#"{ shell = "a", env = { A = true } }"#).is_err(),
            "non-string env"
        );
    }
}
