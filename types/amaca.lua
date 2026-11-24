---Data passed to hooks and to the prompt function.
---@class amaca.Ctx
---@field id string Ticket id.
---@field worktree string Absolute path of the worktree.
---@field mode "solve"|"review"
---@field created boolean `true` when this run created the worktree.

---Command with an argv array part: run directly, without a shell.
---@class amaca.ArgvCommand: string[]
---@field cwd? string Working directory, relative to the current directory.
---@field env? table<string, string> Added to the inherited environment.

---Command run through `sh -c` (`cmd /C` on Windows).
---@class amaca.ShellCommand
---@field shell string
---@field cwd? string Working directory, relative to the current directory.
---@field env? table<string, string> Added to the inherited environment.

---A shell line, an argv, or a table with `shell`.
---@alias amaca.Command string|amaca.ArgvCommand|amaca.ShellCommand

---@alias amaca.Hook fun(ctx: amaca.Ctx)

---Prompt sent to Claude. A string has `{id}` replaced with the ticket id; a
---function returns the prompt and no substitution is applied to its result.
---@alias amaca.Prompt string|fun(ctx: amaca.Ctx): string

---@class amaca.Hooks
---@field created? amaca.Hook After entering the worktree; `solve` only when it was just created, `review` always.
---@field enter? amaca.Hook After `created`, before the pane layout.
---@field before_launch? amaca.Hook After the pane layout, right before Claude starts.

---Table returned by `.amaca.lua`.
---@class amaca.Config
---@field prompt? amaca.Prompt
---@field hooks? amaca.Hooks

---@class amaca.Api
---Runs a command, inheriting the terminal. Raises an error on a non-zero exit.
---@field run fun(cmd: amaca.Command)
---Home directory of the current user (`$HOME`, `%USERPROFILE%` on Windows).
---Raises an error when it cannot be determined.
---@field home fun(): string

-- Only comments and this no-op line: the file is safe to paste at the top of a
-- `.amaca.lua`, where it must not redefine the real `amaca`.
---@type amaca.Api
amaca = amaca
