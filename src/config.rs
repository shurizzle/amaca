use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use mlua::{Function, Lua, Table, Value};

use crate::command::{self, Run};

const FILE_NAME: &str = ".amaca.lua";

/// LuaLS annotations, printed by `amaca types`.
pub const TYPES: &str = include_str!("../types/amaca.lua");

const DEFAULT_CREATED: &str = "The worktree was just created: do any setup it needs. \
     Solve the ticket {id}. The ticket is already in this session's context. \
     Follow the repository conventions.";
const DEFAULT_EXISTING: &str = "Solve the ticket {id}. The ticket is already in this \
     session's context. Follow the repository conventions.";
const DEFAULT_REVIEW: &str = "Review the pull request checked out in this worktree: {id}. \
     The Jira ticket and the whole GitHub pull request conversation are already in this \
     session's context: use them as the handoff. Report findings; do not change code.";

/// Which subcommand is running.
#[derive(Clone, Copy)]
pub enum Mode {
    Solve,
    Review,
}

/// Data handed to hooks and prompt functions as a Lua table.
pub struct Ctx {
    pub id: String,
    pub worktree: PathBuf,
    pub mode: Mode,
    /// The worktree was created by this run.
    pub created: bool,
}

#[derive(Default)]
struct Hooks {
    created: Option<Function>,
    enter: Option<Function>,
    before_launch: Option<Function>,
}

/// User configuration, loaded from `.amaca.lua` files. Every field is
/// optional, so parents and children merge field by field.
pub struct Config {
    lua: Lua,
    prompt: Option<Value>,
    hooks: Hooks,
}

impl Config {
    /// Loads the configuration from the current directory and its ancestors.
    /// The file closest to the current directory wins field by field.
    pub fn load() -> Result<Self> {
        let start = std::env::current_dir().context("cannot read the current directory")?;
        load_from(&start)
    }

    /// The prompt sent to Claude: the configured one, or the built-in default
    /// for the mode and for whether the worktree was just created.
    pub fn prompt(&self, ctx: &Ctx) -> Result<String> {
        let default = match (ctx.mode, ctx.created) {
            (Mode::Review, _) => DEFAULT_REVIEW,
            (Mode::Solve, true) => DEFAULT_CREATED,
            (Mode::Solve, false) => DEFAULT_EXISTING,
        };
        self.resolve(&self.prompt, default, ctx)
    }

    /// Runs a hook if the configuration defines it.
    pub fn hook(&self, name: &str, ctx: &Ctx) -> Result<()> {
        let hook = match name {
            "created" => &self.hooks.created,
            "enter" => &self.hooks.enter,
            "before_launch" => &self.hooks.before_launch,
            _ => bail!("unknown hook {name:?}"),
        };
        if let Some(hook) = hook {
            hook.call::<()>(self.ctx_table(ctx)?)
                .with_context(|| format!("hook {name:?} failed"))?;
        }
        Ok(())
    }

    /// A string prompt has `{id}` replaced; a function receives the context
    /// and returns the prompt.
    fn resolve(&self, value: &Option<Value>, default: &str, ctx: &Ctx) -> Result<String> {
        match value {
            None => Ok(default.replace("{id}", &ctx.id)),
            Some(Value::String(text)) => Ok(text.to_str()?.replace("{id}", &ctx.id)),
            Some(Value::Function(f)) => f
                .call::<String>(self.ctx_table(ctx)?)
                .context("prompt function failed"),
            Some(_) => unreachable!("prompt values are checked at load"),
        }
    }

    fn ctx_table(&self, ctx: &Ctx) -> Result<Table> {
        let table = self.lua.create_table()?;
        table.set("id", ctx.id.as_str())?;
        table.set(
            "worktree",
            ctx.worktree
                .to_str()
                .context("worktree path is not valid UTF-8")?,
        )?;
        table.set(
            "mode",
            match ctx.mode {
                Mode::Solve => "solve",
                Mode::Review => "review",
            },
        )?;
        table.set("created", ctx.created)?;
        Ok(table)
    }

    /// Fills the fields still unset, so the closest file wins.
    fn fill_from(&mut self, other: (Option<Value>, Hooks)) {
        let (prompt, hooks) = other;
        self.prompt = self.prompt.take().or(prompt);
        self.hooks.created = self.hooks.created.take().or(hooks.created);
        self.hooks.enter = self.hooks.enter.take().or(hooks.enter);
        self.hooks.before_launch = self.hooks.before_launch.take().or(hooks.before_launch);
    }
}

fn load_from(start: &Path) -> Result<Config> {
    let lua = Lua::new();
    register_api(&lua)?;
    let mut config = Config {
        lua,
        prompt: None,
        hooks: Hooks::default(),
    };
    let mut dir = Some(start);
    while let Some(current) = dir {
        let path = current.join(FILE_NAME);
        match fs::read_to_string(&path) {
            Ok(text) => {
                let layer = parse(&config.lua, &text, &path)
                    .with_context(|| format!("cannot load {}", path.display()))?;
                config.fill_from(layer);
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err).with_context(|| format!("cannot read {}", path.display()));
            }
        }
        dir = current.parent();
    }
    Ok(config)
}

/// Registers the global `amaca` table: `amaca.run(cmd)` and `amaca.home()`.
fn register_api(lua: &Lua) -> Result<()> {
    let api = lua.create_table()?;
    api.set(
        "run",
        lua.create_function(|_, cmd: Value| {
            Run::from_lua(cmd)
                .and_then(|run| command::run(&run))
                .map_err(|err| mlua::Error::runtime(format!("{err:#}")))
        })?,
    )?;
    api.set(
        "home",
        lua.create_function(|_, ()| {
            std::env::home_dir()
                .and_then(|home| home.into_os_string().into_string().ok())
                .ok_or_else(|| mlua::Error::runtime("cannot determine the home directory"))
        })?,
    )?;
    lua.globals().set("amaca", api.clone())?;
    // `local amaca = require("amaca")` returns the same table as the global.
    lua.globals()
        .get::<Table>("package")?
        .get::<Table>("loaded")?
        .set("amaca", api)?;
    Ok(())
}

fn parse(lua: &Lua, text: &str, path: &Path) -> Result<(Option<Value>, Hooks)> {
    let value: Value = lua
        .load(text)
        .set_name(format!("@{}", path.display()))
        .eval()?;
    let root = match value {
        Value::Nil => return Ok(Default::default()),
        Value::Table(root) => root,
        other => bail!("the file must return a table, got {}", other.type_name()),
    };
    check_keys(&root, &["prompt", "hooks"], "config")?;

    let prompt = match root.get::<Value>("prompt")? {
        Value::Nil => None,
        value @ (Value::String(_) | Value::Function(_)) => Some(value),
        other => bail!(
            "prompt must be a string or a function, got {}",
            other.type_name()
        ),
    };

    let mut hooks = Hooks::default();
    if let Some(table) = root.get::<Option<Table>>("hooks")? {
        check_keys(&table, &["created", "enter", "before_launch"], "hooks")?;
        hooks.created = table
            .get("created")
            .context("hooks.created must be a function")?;
        hooks.enter = table
            .get("enter")
            .context("hooks.enter must be a function")?;
        hooks.before_launch = table
            .get("before_launch")
            .context("hooks.before_launch must be a function")?;
    }
    Ok((prompt, hooks))
}

/// Rejects keys outside `allowed`, to catch typos.
fn check_keys(table: &Table, allowed: &[&str], what: &str) -> Result<()> {
    for pair in table.pairs::<Value, Value>() {
        let (key, _) = pair?;
        match &key {
            Value::String(name) if allowed.contains(&&*name.to_str()?) => {}
            Value::String(name) => bail!("unknown key {:?} in {what}", name.to_str()?.to_string()),
            _ => return Err(anyhow!("{what} keys must be strings")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use uuid::Uuid;

    use super::{Ctx, Mode, load_from};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("amaca-config-{}-{name}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    fn write(dir: &Path, text: &str) {
        fs::write(dir.join(".amaca.lua"), text).expect("write config");
    }

    fn ctx(dir: &Path) -> Ctx {
        Ctx {
            id: "ABC-1".into(),
            worktree: dir.to_owned(),
            mode: Mode::Solve,
            created: true,
        }
    }

    #[test]
    fn closest_file_wins_field_by_field() {
        let root = scratch("priority");
        let child = root.join("child");
        fs::create_dir_all(&child).expect("create child dir");
        write(
            &root,
            r#"return { prompt = "parent {id}", hooks = { enter = function() end } }"#,
        );
        write(&child, r#"return { prompt = "child {id}" }"#);

        let config = load_from(&child).expect("load config");
        assert_eq!(config.prompt(&ctx(&child)).unwrap(), "child ABC-1");
        assert!(
            config.hooks.enter.is_some(),
            "hook inherited from the parent"
        );
    }

    #[test]
    fn prompt_function_receives_ctx() {
        let dir = scratch("prompt-fn");
        write(
            &dir,
            r#"return { prompt = function(ctx) return ctx.id .. ":" .. ctx.mode .. ":" .. tostring(ctx.created) end }"#,
        );
        let config = load_from(&dir).expect("load config");
        assert_eq!(config.prompt(&ctx(&dir)).unwrap(), "ABC-1:solve:true");
    }

    #[test]
    #[cfg(unix)]
    fn hooks_run_with_ctx_and_amaca_run() {
        let dir = scratch("hooks");
        let marker = dir.join("marker");
        write(
            &dir,
            r#"return { hooks = { created = function(ctx)
                    amaca.run({ shell = "printf '%s' \"$X\" > marker", cwd = ctx.worktree, env = { X = ctx.id } })
                end } }"#,
        );
        let config = load_from(&dir).expect("load config");
        config.hook("created", &ctx(&dir)).expect("hook runs");
        assert_eq!(fs::read_to_string(marker).unwrap(), "ABC-1");
        config
            .hook("enter", &ctx(&dir))
            .expect("missing hook is a no-op");
    }

    #[test]
    fn hook_errors_propagate() {
        let dir = scratch("hook-error");
        write(
            &dir,
            r#"return { hooks = { enter = function() amaca.run("exit 3") end } }"#,
        );
        let config = load_from(&dir).expect("load config");
        assert!(config.hook("enter", &ctx(&dir)).is_err());
    }

    #[test]
    fn rejects_unknown_keys_and_bad_types() {
        for (name, text) in [
            ("top", "return { hook = {} }"),
            (
                "hook-name",
                "return { hooks = { created_ = function() end } }",
            ),
            ("hook-type", r#"return { hooks = { created = "make" } }"#),
            ("prompt-type", "return { prompt = 1 }"),
            ("not-table", "return 1"),
        ] {
            let dir = scratch(name);
            write(&dir, text);
            assert!(load_from(&dir).is_err(), "{name} must be rejected");
        }
    }

    #[test]
    fn reports_syntax_errors_with_path() {
        let dir = scratch("syntax");
        write(&dir, "return {");
        let err = format!("{:#}", load_from(&dir).err().expect("syntax error"));
        assert!(err.contains(".amaca.lua"), "{err}");
    }

    #[test]
    fn types_file_is_safe_to_paste() {
        let lua = mlua::Lua::new();
        super::register_api(&lua).expect("register api");
        let before: mlua::Function = lua.load("return amaca.run").eval().unwrap();
        lua.load(super::TYPES).exec().expect("types file runs");
        let after: mlua::Function = lua.load("return amaca.run").eval().unwrap();
        assert_eq!(before, after, "the types file must not redefine amaca.run");
        let same: bool = lua
            .load(r#"return require("amaca") == amaca"#)
            .eval()
            .unwrap();
        assert!(same);
    }

    #[test]
    fn home_returns_the_home_directory() {
        let lua = mlua::Lua::new();
        super::register_api(&lua).expect("register api");
        let home: String = lua
            .load("return amaca.home()")
            .eval()
            .expect("amaca.home()");
        let expected = std::env::home_dir().expect("home directory");
        assert_eq!(std::path::Path::new(&home), expected);
    }

    #[test]
    fn nil_file_is_empty() {
        let dir = scratch("nil");
        write(&dir, "-- nothing");
        load_from(&dir).expect("load config");
    }

    #[test]
    fn uses_defaults_without_files() {
        let dir = scratch("empty");
        let config = load_from(&dir).expect("load config");
        let mut ctx = ctx(&dir);
        assert!(config.prompt(&ctx).unwrap().contains("just created"));
        ctx.created = false;
        assert!(
            config
                .prompt(&ctx)
                .unwrap()
                .contains("Solve the ticket ABC-1")
        );
        ctx.mode = Mode::Review;
        assert!(
            config
                .prompt(&ctx)
                .unwrap()
                .contains("Review the pull request")
        );
    }
}
