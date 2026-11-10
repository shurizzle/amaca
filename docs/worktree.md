# Worktree

Amaca works on one ticket per git worktree. The branch name is the ticket id.

## Resolution

Given the ticket id `$TICKET` (`$BRANCH`):

1. The branch exists (`refs/heads/$TICKET`):
   - a worktree already has that branch: use it, nothing to create;
   - otherwise create a worktree on that branch.
2. The branch does not exist:
   - dev branch: `git config repo.devBranch`, fallback `develop`;
   - `git fetch origin`;
   - create `$TICKET` from `origin/$DEV` (from the local `$DEV` when the
     remote-tracking ref is missing, as in bare clones);
   - create a worktree on that branch.

## Location

Classic repository (git dir inside the main worktree):

- `.worktrees/` in the main worktree root, worktree at `.worktrees/$TICKET`;
- `.worktrees/` is added to the main worktree's `.gitignore`.

Bare repository (detected with `git rev-parse --is-bare-repository` on the
shared git directory from `git rev-parse --git-common-dir`, so a worktree of a
bare clone counts too):

- worktree as a sibling of the git dir, named `$TICKET`.

The main worktree root and the existing worktrees come from
`git worktree list --porcelain`, never from path heuristics.

## Review worktrees

`review` uses a throwaway worktree instead of the ticket one:

- directory and branch are `review-<ticket>`;
- the start point is `origin/<head branch>`, fetched with
  `git fetch origin <head branch>`, falling back to the local branch when the
  remote-tracking ref is missing;
- when the branch or the path already exists, a random suffix
  (`review-<ticket>-<8 hex>`) avoids the collision;
- the worktree is removed with `git worktree remove --force` when Claude
  exits, then the branch is deleted with `git branch -D`. The branch is kept
  only when the worktree removal fails.

It lives in the same location as the ticket worktree (`.worktrees/` or the
bare repository's sibling).

## Rules

- Never touch a worktree that belongs to another ticket.
- Running Amaca twice on the same ticket reuses the worktree.
- When it creates the worktree, Amaca runs the `command.created` config command
  inside it, in `solve` and in `review`; its failure stops the run
  ([docs/config.md](config.md)).
- Amaca changes its working directory into the ticket worktree, so every later
  step (shell, Claude) starts there.
- Do not delete or prune worktrees. Removal stays manual.
