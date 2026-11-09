# iTerm2

iTerm2 is a terminal with a built-in multiplexer. Amaca drives it through the
iTerm2 API when it runs inside iTerm2 and no standalone multiplexer is
available: zellij, tmux and screen take precedence over iTerm2 when more than
one is present ([docs/multiplexer.md](multiplexer.md)).

The backend is compiled on macOS only.

iTerm2 has no CLI for panes, so the backend speaks its API directly: a
WebSocket with protobuf messages over
`~/Library/Application Support/iTerm2/private/socket`. The messages Amaca
uses are hand-written in `src/mux/iterm2.rs`, from `proto/api.proto` in the
iTerm2 repository.

## Setup

- Enable Settings > General > Magic > Enable Python API.
- The first run asks for permission: iTerm2 shows its own dialog, and macOS
  asks whether the terminal running Amaca may control iTerm2. Allow both. A
  refusal shows `cannot request the iTerm2 cookie` with the fix.

## Check

`ITERM_SESSION_ID` must be set and the control socket must exist. When
`ITERM_SESSION_ID` is set but the socket is missing, Amaca warns that the
Python API is off and falls through to the next backend.

## Layout

1. Read cookie and key from `ITERM2_COOKIE` and `ITERM2_KEY`, or request them
   from iTerm2 with an Apple Event (`request cookie and key for app named
   "amaca"`). No `osascript`.
2. Connect to the socket and send `SplitPaneRequest`: horizontal, before the
   current session, with the command and the worktree as custom profile
   properties. The new pane runs the hidden `ꙮ` command with the ticket id,
   title and summary.
3. `ListSessions`, then `SetTabLayout`: the banner pane gets 2 rows and the
   pane it was split from takes the rows back. When iTerm2 rejects the
   layout, Amaca prints a warning and keeps the split as it is.
4. `ActivateRequest` on the original session gives the focus back.

`ꙮ` sets the terminal title to the ticket id, prints the ticket data and waits
forever, so the banner stays visible above Claude.

## Command line

iTerm2 evaluates a profile command as an interpolated string and then parses
it like a shell line. Amaca takes both layers into account: each argument is
single-quoted, then every backslash and double quote is escaped for the
interpolation. Ticket text with quotes, backslashes or shell syntax arrives
intact.

## Rules

- Only the iTerm2 API. No `osascript`, no AppleScript.
- Keep the socket, the protocol messages and the Apple Event inside
  `src/mux/iterm2.rs`.
- Running Amaca twice on the same ticket currently adds another top pane; the
  split is not reused. Reuse is open.
