# GitHub

Amaca does not talk to GitHub. No REST client, no token handling.

Amaca passes the pull request to Claude. Claude reads it through its GitHub
MCP.

## Review

`amaca review <link|number>` runs this flow:

1. Claude resolves the pull request head branch with the GitHub MCP tools.
   The branch may be the ticket id or add a title
   (`AA-1234-ticket-title`).
2. Amaca extracts the Jira ticket key from the branch: alphanumeric tokens are
   compared in pairs, and the first pair shaped like `PROJECT-123` wins. Only
   the key goes to the ticket fetch ([docs/jira.md](jira.md)).
3. The ticket fetch runs in the run's Claude session, as in `solve`
   ([docs/jira.md](jira.md)).
4. Amaca resumes that session and asks Claude to load the whole pull request
   conversation: description, commits, changed files, reviews, review comments
   and issue comments. The conversation stays in the session's context.
5. Amaca creates the throwaway review worktree on the head branch
   ([docs/worktree.md](worktree.md)).
6. Amaca runs the `command.created` setup command from the config inside it
   ([docs/config.md](config.md)).
7. Amaca opens the same banner layout as `solve` and resumes the session with
   the `review` prompt ([docs/config.md](config.md)). The prompt uses the
   ticket and the conversation already in context as the handoff.

## MCP rules

- GitHub data comes from Claude, never from Amaca.
- The allowed rules are `mcp__<server>` for every MCP server that
  `claude mcp list` reports: neither the server nor its tools are hardcoded.
  When the listing fails, Amaca stops.
- The pull request must live on `origin`: Amaca runs
  `git fetch origin <head branch>`. Fork heads are not handled.
- The review worktree and its local branch are removed when Claude exits: the
  next review of the same ticket gets the plain `review-<ticket>` name again
  ([docs/worktree.md](worktree.md)).
