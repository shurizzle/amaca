# screen

screen is a standalone terminal multiplexer: it runs inside any terminal. It
owns the panes even if the outer terminal has its own multiplexer, so screen
takes precedence over WezTerm and comes after zellij and tmux
([docs/multiplexer.md](multiplexer.md)).

## Check

`STY` must be set and `screen -Q number` must succeed. When the probe fails,
Amaca falls through to the next backend; with none reachable it exits before
fetching the ticket or creating the worktree.

The backend needs screen 4.2 or newer: it uses the directional `focus`
subcommands and the `-Q` query option.

## Layout

1. `screen -X split` splits the current region and opens a new region below
   it; the focus stays on the current region.
2. `screen -X focus down` moves the focus into the new region.
3. `screen -X resize 4` shrinks the new region: 3 content rows plus the
   caption line screen adds to split displays.
4. `screen -X screen -t <id> sh -c "cd <worktree> && exec <current exe> ꙮ <id> <title> <summary>"`
   creates a window with the hidden `ꙮ` command and selects it in the new
   region. The new region shows the internal blank window, which runs
   nothing; the shell wrapper is needed because screen has no cwd option for
   a new window.
5. `screen -X focus up` gives the focus back to the original region, where
   Claude launches.

`ꙮ` sets the terminal title to the ticket id, prints the ticket data and waits
forever, so the banner stays visible next to Claude.

Unlike the other backends, screen cannot move a region: the banner region
stays below the original one. Its caption shows the ticket id.

## Rules

- Only the `screen` CLI. No direct socket protocol, no writing screen config
  files.
- Keep `screen` calls inside `src/mux/screen.rs`.
- Running Amaca twice on the same ticket currently adds another region; the
  split is not reused.
