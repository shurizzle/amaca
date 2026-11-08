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
  // Only the prompts that differ: the rest keeps the defaults.
  "prompt": {
    "created": "Fresh worktree: install the dependencies, then solve {id}.",
    "existing": "Solve {id}, following the repository conventions."
  }
}
```

## Rules

- Comments and trailing commas are accepted (JSONC).
- An unknown key is an error, so a typo cannot be silently ignored.
- The prompt is resolved from the directory Amaca is started in, before it
  enters the ticket worktree.
