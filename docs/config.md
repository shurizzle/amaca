# Configuration

Amaca reads `.amaca.lua` from the current directory and every ancestor, up to
the filesystem root. Each file returns a table. The file closest to the current
directory wins, field by field: a child overrides only the keys it writes, and
anything it omits keeps the value from the nearest parent that sets it. All
files run in one Lua state (LuaJIT, through `mlua`), so functions from any
ancestor stay callable.

The file is trusted code, like a shell rc: it runs with the full Lua standard
library (`os`, `io`...), without a sandbox.

```lua
return {
  -- Only what differs: the rest keeps the defaults.
  prompt = function(ctx)
    if ctx.mode == "review" then
      return "Review the pull request for " .. ctx.id .. ", report only."
    end
    if ctx.created then
      return "Fresh worktree: install the dependencies, then solve " .. ctx.id .. "."
    end
    return "Solve " .. ctx.id .. ", following the repository conventions."
  end,
  hooks = {
    created = function(ctx)
      amaca.run("npm install")
      amaca.run({ "cargo", "build", cwd = "backend", env = { RUSTFLAGS = "-Dwarnings" } })
    end,
  },
}
```

## Prompt

`prompt` is a string or a function. In a string, `{id}` is replaced with the
ticket id. A function receives `ctx` and returns the prompt as a string; no
substitution is applied to its result. It is used for every run: tell the cases
apart with `ctx.mode` (`"solve"` or `"review"`) and `ctx.created`.

Without `prompt` Amaca uses a built-in default, chosen by the same fields: one
for a just-created worktree, one for an existing worktree, one for `review`
(`src/config.rs`).

## Hooks

Each key of `hooks` is the name of a hook and its value is a function that
receives `ctx`:

| Hook | When | `solve` | `review` |
| --- | --- | --- | --- |
| `created` | After Amaca enters the worktree, before the pane layout | Only when the worktree was just created | Always: `review` always creates its worktree |
| `enter` | After `created`, before the pane layout | Always | Always |
| `before_launch` | After the pane layout, right before Claude starts | Always | Always |

Hooks run with the worktree as the current directory. An error raised by a
hook, including a failed `amaca.run`, stops Amaca.

### `ctx`

| Field | Value |
| --- | --- |
| `ctx.id` | Ticket id |
| `ctx.worktree` | Absolute path of the worktree |
| `ctx.mode` | `"solve"` or `"review"` |
| `ctx.created` | `true` when this run created the worktree |

Prompt functions receive the same `ctx`.

`require("amaca")` returns the same table as the global `amaca`.

### `amaca.home()`

Returns the home directory as a string (`$HOME`, `%USERPROFILE%` on Windows);
raises a Lua error when it cannot be determined.

### `amaca.run(cmd)`

Runs a command, inheriting the terminal, and raises a Lua error on a non-zero
exit. `cmd` is:

- a string: a shell line, run with `sh -c`, or `cmd /C` on Windows;
- a table with an array part: an argv run directly, without a shell
  (`{ "make", "setup" }`);
- a table with `shell = "..."`: a shell line.

A table can also hold `cwd` (relative to the current directory) and `env` (a
table of strings, added to the inherited environment). The array part and
`shell` are mutually exclusive, and one of them is required.

## Editor types

`types/amaca.lua` holds [LuaLS](https://luals.github.io/) annotations for the
`amaca` global, `ctx` and the returned table. Config files live outside any
project, so put the folder in the language server library at the user level,
not in a per-project `.luarc.json`:

```lua
-- Neovim
vim.lsp.config("lua_ls", {
  settings = { Lua = {
    runtime = { version = "LuaJIT" },
    workspace = { library = { "/path/to/amaca/types" } },
  } },
})
```

```json
// VS Code settings.json
"Lua.runtime.version": "LuaJIT",
"Lua.workspace.library": ["/path/to/amaca/types"]
```

Other editors: pass the same keys (`runtime.version`, `workspace.library`) in a
JSON file with `lua-language-server --configpath <file>`.

LuaLS has no per-file way to load a library. Without editor configuration,
`amaca types` prints the same annotations, embedded in the binary. They are
only comments plus a no-op line, so paste them at the top of a config file:

```sh
amaca types | cat - .amaca.lua > .amaca.lua.new && mv .amaca.lua.new .amaca.lua
```

Annotate the returned table to get completion and checks on the keys:

```lua
---@type amaca.Config
return {
  hooks = {
    created = function(ctx) amaca.run("npm install") end,
  },
}
```

## Rules

- An unknown key, in the file, in `hooks` or in an `amaca.run`
  table, is an error, so a typo cannot be silently ignored.
- A file that returns `nil` is empty.
- The config is loaded from the directory Amaca is started in, before it enters
  the ticket worktree.
