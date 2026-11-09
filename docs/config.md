# Configuration

Amaca reads `.amaca.jsonc` (JSON with comments) from the current directory and
every ancestor, up to the filesystem root. The file closest to the current
directory wins, field by field: a child overrides only the keys it writes, and
anything it omits keeps the value from the nearest parent that sets it.

## Prompts

| Key | Prompt |
| --- | --- |
| `prompt.created` | The worktree was just created |
| `prompt.existing` | The worktree already existed |

`{id}` is replaced with the ticket id. Without a config file Amaca uses the
built-in defaults in `src/config.rs`.

```jsonc
{
  // Only what differs: the rest keeps the defaults.
  "prompt": {
    "created": "Fresh worktree: install the dependencies, then solve {id}.",
    "existing": "Solve {id}, following the repository conventions."
  },
  "command": {
    // Shell line, or argv run without a shell: ["npm", "install"].
    "created": "npm install"
  }
}
```

## Commands

| Key | When |
| --- | --- |
| `command.created` | The worktree was just created |

A string runs through `system(3)`, so the shell interprets it. An array of
strings is the argv run directly with `Command`, without a shell. Both inherit
the terminal, run after Amaca enters the new worktree and before the WezTerm
layout and Claude start. The command does not run when the worktree already
existed. A non-zero exit stops Amaca.

## Editor schema

`schema.json` in the repository root describes this file. Point your editor at
it with a `$schema` key inside the config:

```jsonc
{
  "$schema": "../schema.json",
  "prompt": { "existing": "Solve {id}, following the repository conventions." }
}
```

`$schema` is accepted and ignored. With VS Code you can instead associate the
file globally in `.vscode/settings.json`:

```json
{
  "json.schemas": [
    {
      "fileMatch": ["**/.amaca.jsonc"],
      "url": "./schema.json"
    }
  ]
}
```

The schema is generated from the Rust types with `schemars`, under
`#[cfg_attr(test, derive(schemars::JsonSchema))]`, so the dependency stays a
dev-dependency. After changing a config struct, regenerate it and commit the
result:

```sh
UPDATE_SCHEMA=1 cargo test config_schema_is_current
```

`cargo test` fails when the committed `schema.json` is stale.

## Rules

- Comments and trailing commas are accepted (JSONC).
- An unknown key is an error, so a typo cannot be silently ignored.
- The prompt is resolved from the directory Amaca is started in, before it
  enters the ticket worktree.
