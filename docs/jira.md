# Jira

Amaca does not talk to Jira. No REST client, no token handling.

Amaca passes the ticket key to Claude. Claude reads the ticket through its
Atlassian MCP.

## Rules

- Ticket data comes from Claude, never from Amaca.
- Amaca only handles the key/context it needs to name the window and the
  working directory.
- Amaca asks Claude for the ticket through the Atlassian MCP tools
  (`claude --print --allowedTools=mcp__atlassian --session-id <uuid>`). Claude
  returns one JSON object: `id`, `title`, `summary`, `body`.
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

## Open questions

- What exactly is passed to Claude: only the key, or also a prompt template
  with the expected output. Not decided yet.
