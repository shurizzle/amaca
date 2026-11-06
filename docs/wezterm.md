# WezTerm

Amaca drives a terminal multiplexer. **WezTerm is the only supported backend
for now**, chosen for simplicity.

## Future backends

Other multiplexers will be supported later (tmux, for example). Do not couple
the rest of the code to WezTerm: the input parser, ticket resolution and
launchers must not care which multiplexer is running. Keep WezTerm-specific
code — `wezterm cli` calls, pane/tab/workspace creation — inside the WezTerm
backend.

## Layout

Amaca runs inside a WezTerm pane (`WEZTERM_PANE`), and builds the layout with
`wezterm cli`:

1. `wezterm cli split-pane --top --cells 2 --cwd <worktree> -- <current exe> ꙮ <id> <title> <summary>`
   splits the current pane horizontally; the new 2-row pane above runs the
   hidden `ꙮ` command with the ticket id, title and summary.
2. `wezterm cli activate-pane --pane-id <WEZTERM_PANE>` gives the focus back to
   the original pane.
3. Claude launches in the original pane.

`ꙮ` sets the terminal title to the ticket id, prints the ticket data and waits
forever, so the banner stays visible above Claude.

After changing directory into the ticket worktree, Amaca emits OSC 7
(`\x1b]7;file://<path>\x07`) so the terminal tracks the worktree as the current
directory.

## Rules

- Only `wezterm cli`. No direct socket protocol, no writing WezTerm
  config files.
- Keep `wezterm cli` calls inside `src/wezterm.rs`.
- Do not add a multiplexer abstraction now. Add it when the second backend
  arrives.
- Running Amaca twice on the same ticket currently adds another top pane; the
  split is not reused. Reuse is open.

## Open questions

- Reusing an existing top pane instead of splitting again.
- Backend selection: config file, flag, or detection. Not decided yet.
