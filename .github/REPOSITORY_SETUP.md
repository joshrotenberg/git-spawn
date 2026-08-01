# Repository automation setup

## Release pull-request checks

The `Release` workflow keeps release-plz on the repository-scoped
`GITHUB_TOKEN`. GitHub prevents most events produced with that token from
starting another unattended workflow, so merely updating the release branch
does not reliably produce merge checks.

After release-plz creates or updates its pull request, the workflow reads the
action's structured `head_branch` and `number` outputs and explicitly dispatches
`CI` and `PR Title` at that branch. GitHub treats `workflow_dispatch` as an
exception to the recursion guard, and a dispatch at a branch uses that branch's
latest commit as `GITHUB_SHA`. The resulting `Required checks` and
`Conventional PR title` results therefore belong to the exact release candidate
revision without a personal access token or stored GitHub App credential.

The workflow needs `actions: write` to create those dispatches. Its remaining
permissions are unchanged: `contents: write` and `pull-requests: write` allow
release-plz to maintain the release pull request. No additional Actions secret
is required.

## Protecting `main`

Create or update one `main` ruleset to match
[rulesets/main.json](rulesets/main.json). This repository already has a `main`
ruleset, so update that rule in place; if it is absent in a restored or forked
repository, import the JSON as a new active ruleset. Do not leave two
overlapping rulesets. Require both stable check names:

- `Required checks` is the rollup for every matrix and non-matrix job in
  `ci.yml`.
- `Conventional PR title` protects the squash-merge commit format used by
  release-plz.

The ruleset requires an up-to-date pull request, resolved review conversations,
and squash merge; it also blocks deletion and force-pushes. Repository
administrators have the deliberate emergency bypass (`RepositoryRole` id 5).
Use that bypass only for incident recovery, record the reason, and immediately
restore the branch to a state that passes both checks.

Do not merge the current release pull request merely to test this setup.
Publishing remains held until the planned implementation, API hardening, and
documentation work is complete.

## Safe verification

1. Merge the workflow changes before activating the required-status-check rule,
   so both named contexts exist on `main` first.
2. Confirm the resulting `Release` run updates release PR #59 and dispatches
   both workflows at its head branch without manual approval.
3. Confirm the pull request's check rollup reports that exact head SHA, and that
   `Required checks` stays pending until all CI jobs finish.
4. Update the existing `main` ruleset from `rulesets/main.json`.
5. Confirm GitHub reports a pull request behind `main` as blocked and requires
   it to be updated, and that a failed required check blocks merge.

Do not merge release PR #59 and do not run a publish operation as part of
verification.
