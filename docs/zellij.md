# zellij

zellij is a standalone terminal multiplexer: it runs inside any terminal. It
often runs inside WezTerm or iTerm2, where it owns the panes, so zellij takes
precedence over tmux, screen, iTerm2 and WezTerm
([docs/multiplexer.md](multiplexer.md)).

## Check

`ZELLIJ_PANE_ID` must be set and `zellij action list-panes` must succeed. When
the probe fails, Amaca falls through to the next backend; with none reachable
it exits before fetching the ticket or creating the worktree.

## Layout

1. `zellij action new-pane --direction down --cwd <worktree> -- <current exe> ꙮ <id> <title> <summary>`
   splits the current pane and places the new pane below it; that pane runs the
   hidden `ꙮ` command with the ticket id, title and summary.
2. `zellij action move-pane --pane-id <new pane> up` swaps the new pane above
   the original one: `new-pane` can only split right or down.
3. `zellij action resize --pane-id <new pane> decrease down` shrinks the new
   pane one step at a time until zellij stops: `new-pane` has no size option
   and opens at half height, and zellij keeps a minimum pane height (6 rows
   with the frame in zellij 0.45.1).
4. `zellij action focus-pane-id <ZELLIJ_PANE_ID>` gives the focus back to the
   original pane.
5. Claude launches in the original pane.

`ꙮ` sets the terminal title to the ticket id, prints the ticket data and waits
forever, so the banner stays visible above Claude.

## Rules

- Only the `zellij` CLI. No direct socket protocol, no writing zellij config
  files.
- Keep `zellij action` calls inside `src/mux/zellij.rs`.
- Running Amaca twice on the same ticket currently adds another top pane; the
  split is not reused.
