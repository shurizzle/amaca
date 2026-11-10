# Jira

Amaca does not talk to Jira. No REST client, no token handling.

Amaca passes the ticket key to Claude. Claude reads the ticket through its
Atlassian MCP.

## Rules

- Ticket data comes from Claude, never from Amaca.
- Amaca only handles the key/context it needs to name the window and the
  working directory.
- Amaca asks Claude for the ticket through the Atlassian MCP tools
  (`claude --print --allowedTools=<rules> --session-id <uuid>`). The allowed
  tools are `mcp__<server>__getJiraIssue` for every MCP server that
  `claude mcp list` reports, so no client-specific server name is hardcoded.
  Claude returns one JSON object: `id`, `title`, `summary`, `body`.
  - `claude mcp list` has no JSON output: the parser in `src/ticket.rs` reads
    the first token of each unindented line and drops disabled servers.
  - When the listing fails, Amaca stops: it does not guess a server name.
- Amaca keeps only `id`, `title` and `summary` (`Ticket` in `src/ticket.rs`).
  `body` is not parsed: it stays in the Claude session's context, which later
  steps resume, so the content is never fetched twice.
- The fetch happens inside the Claude session that the launchers resume
  (`--resume <uuid>`). The ticket content is already in its context, so later
  steps must not fetch it again.
- `body`:
  - subtask of any type: parent content followed by the subtask content;
  - anything else: the ticket content.
- `summary`: one line of at most 10 words that adds what the title does not —
  purpose, affected area, expected outcome or constraints, taken from the
  complete body. Never repeats or paraphrases the title.
- No Jira ticket found is an explicit error.
- Never write client names in configs or docs.
- The prompt is a template in `src/ticket.rs`: it asks for the key, the fetch
  through the Atlassian MCP, the `body` and `summary` rules, one JSON object
  with that schema, and `NONE` when no ticket exists.
