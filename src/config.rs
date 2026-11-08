use std::ffi::CString;
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context, Result};
use jsonc_parser::parse_to_serde_value;
use serde::{Deserialize, Deserializer};

const FILE_NAME: &str = ".amaca.jsonc";

const DEFAULT_CREATED: &str = "The worktree was just created: do any setup it needs. \
     Solve the ticket {id}. The ticket is already in this session's context. \
     Follow the repository conventions.";
const DEFAULT_EXISTING: &str = "Solve the ticket {id}. The ticket is already in this \
     session's context. Follow the repository conventions.";

/// User configuration. Every field is optional, so parents and children merge
/// field by field.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub prompt: Prompt,
    pub command: Command,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Prompt {
    /// Prompt used when the worktree was just created.
    pub created: Option<String>,
    /// Prompt used when the worktree already existed.
    pub existing: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Command {
    /// Shell command run when the worktree was just created.
    #[serde(deserialize_with = "deserialize_cstring")]
    pub created: Option<CString>,
}

/// A NUL byte cannot live in a `CString`, so the config is validated here.
fn deserialize_cstring<'de, D>(deserializer: D) -> Result<Option<CString>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(CString::new)
        .transpose()
        .map_err(serde::de::Error::custom)
}

impl Config {
    /// Loads the configuration from the current directory and its ancestors.
    /// The file closest to the current directory wins field by field.
    pub fn load() -> Result<Self> {
        let start = std::env::current_dir().context("cannot read the current directory")?;
        load_from(&start)
    }

    /// The interactive prompt; `{id}` is replaced with the ticket id.
    pub fn prompt(&self, created: bool, ticket: &str) -> String {
        let prompt = if created {
            self.prompt.created.as_deref().unwrap_or(DEFAULT_CREATED)
        } else {
            self.prompt.existing.as_deref().unwrap_or(DEFAULT_EXISTING)
        };
        prompt.replace("{id}", ticket)
    }

    /// Fills the fields still unset, so the closest file wins.
    fn fill_from(&mut self, other: Config) {
        self.prompt.created = self.prompt.created.take().or(other.prompt.created);
        self.prompt.existing = self.prompt.existing.take().or(other.prompt.existing);
        self.command.created = self.command.created.take().or(other.command.created);
    }
}

fn load_from(start: &Path) -> Result<Config> {
    let mut config = Config::default();
    let mut dir = Some(start);
    while let Some(current) = dir {
        let path = current.join(FILE_NAME);
        match fs::read_to_string(&path) {
            Ok(text) => config.fill_from(parse(&text, &path)?),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err).with_context(|| format!("cannot read {}", path.display()));
            }
        }
        dir = current.parent();
    }
    Ok(config)
}

fn parse(text: &str, path: &Path) -> Result<Config> {
    let config: Option<Config> = parse_to_serde_value(text, &Default::default())
        .with_context(|| format!("cannot parse {}", path.display()))?;
    Ok(config.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use uuid::Uuid;

    use super::load_from;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("amaca-config-{}-{name}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    fn write(dir: &Path, text: &str) {
        fs::write(dir.join(".amaca.jsonc"), text).expect("write config");
    }

    #[test]
    fn closest_file_wins_field_by_field() {
        let root = scratch("priority");
        let child = root.join("child");
        fs::create_dir_all(&child).expect("create child dir");
        write(
            &root,
            r#"{ "prompt": { "existing": "parent" }, "command": { "created": "setup" } }"#,
        );
        write(
            &child,
            "{\n  // comments and trailing commas are valid JSONC\n  \"prompt\": { \"created\": \"child\", },\n}",
        );

        let config = load_from(&child).expect("load config");
        assert_eq!(config.prompt(true, "ABC-1"), "child");
        assert_eq!(config.prompt(false, "ABC-1"), "parent");
        assert_eq!(
            config.command.created.as_deref(),
            Some(c"setup"),
            "command inherited from the parent file"
        );
    }

    #[test]
    fn rejects_nul_in_command() {
        let dir = scratch("nul");
        write(&dir, r#"{ "command": { "created": "a\u0000b" } }"#);
        assert!(load_from(&dir).is_err(), "NUL cannot live in a CString");
    }

    #[test]
    fn uses_defaults_without_files() {
        let dir = scratch("empty");
        let config = load_from(&dir).expect("load config");
        assert!(config.prompt(true, "ABC-1").contains("just created"));
        assert!(
            config
                .prompt(false, "ABC-1")
                .contains("Solve the ticket ABC-1")
        );
    }
}
