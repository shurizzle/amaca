# WezTerm

WezTerm is a terminal with a built-in multiplexer. Amaca drives it through
`wezterm cli` when it runs inside WezTerm and no standalone multiplexer is
available: zellij, tmux and screen take precedence over WezTerm when more than
one is present ([docs/multiplexer.md](multiplexer.md)).

## Check

`WEZTERM_PANE` must be set and `wezterm cli list` must succeed. When the probe
fails, Amaca exits before fetching the ticket or creating the worktree.

## Layout

1. `wezterm cli split-pane --top --cells 2 --cwd <worktree> -- <current exe> ꙮ <id> <title> <summary>`
   splits the current pane horizontally; the new 2-row pane above runs the
   hidden `ꙮ` command with the ticket id, title and summary.
2. `wezterm cli activate-pane --pane-id <WEZTERM_PANE>` gives the focus back to
   the original pane.
3. Claude launches in the original pane.

`ꙮ` sets the terminal title to the ticket id, prints the ticket data and waits
forever, so the banner stays visible above Claude.

## Rules

- Only `wezterm cli`. No direct socket protocol, no writing WezTerm
  config files.
- Keep `wezterm cli` calls inside `src/mux/wezterm.rs`.
- Running Amaca twice on the same ticket currently adds another top pane; the
  split is not reused. Reuse is open.
