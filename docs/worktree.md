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

Bare repository (detected with `git rev-parse --is-bare-repository`):

- worktree as a sibling of the git dir, named `$TICKET`.

The main worktree root and the existing worktrees come from
`git worktree list --porcelain`, never from path heuristics.

## Rules

- Never touch a worktree that belongs to another ticket.
- Running Amaca twice on the same ticket reuses the worktree.
- Amaca changes its working directory into the ticket worktree, so every later
  step (shell, Claude) starts there.
- Do not delete or prune worktrees. Removal stays manual.
