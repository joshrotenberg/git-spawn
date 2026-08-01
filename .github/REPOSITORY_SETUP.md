# Repository automation setup

## Release pull-request checks

The `Release` workflow keeps release-plz on the repository-scoped
`GITHUB_TOKEN`. GitHub prevents most events produced with that token from
starting another unattended workflow, so merely updating the release branch
does not reliably produce merge checks.

After release-plz creates or updates its pull request, the workflow reads the
action's structured `head_branch` output and explicitly dispatches `CI` and `PR
Title` at that branch. GitHub treats `workflow_dispatch` as an
exception to the recursion guard, and a dispatch at a branch uses that branch's
latest commit as `GITHUB_SHA`. The resulting `Required checks` and
`Conventional PR title` results therefore belong to the exact release candidate
revision without a personal access token or stored GitHub App credential.

A dispatched workflow's check run is attached to its workflow suite, but GitHub
does not include that suite in the release pull request's status rollup. After
each dispatched validation finishes, the workflow therefore publishes a commit
status with the same required context on the exact `GITHUB_SHA`. The status
links back to its originating workflow run and reports success only when the
validation passed; a failed validation publishes failure. Ordinary
`pull_request` runs continue to provide the existing check runs and names and do
not publish duplicate commit statuses. GitHub requires both results to pass when
a check run and commit status with the same name apply to a revision.

The dispatched title check does not trust caller-supplied title text. It resolves
the sole open pull request for `GITHUB_REF_NAME`, requires that pull request's
API head to equal `GITHUB_SHA`, and validates the current title returned by
GitHub.

The release workflow needs `actions: write` to create those dispatches. Its
remaining permissions are unchanged: `contents: write` and `pull-requests:
write` allow release-plz to maintain the release pull request. No additional
Actions secret is required. The dispatched validation workflows grant
`statuses: write` only so they can publish their results; their other
permissions remain read-only.

## Protecting `main`

Create or update one `main` ruleset to match
[rulesets/main.json](rulesets/main.json). This repository already has a `main`
ruleset, so update that rule in place; if it is absent in a restored or forked
repository, import the JSON as a new active ruleset. Do not leave two
overlapping rulesets. Require both stable check names:

For the canonical repository, this configuration was applied in place to
ruleset [`16766350`](https://github.com/joshrotenberg/git-spawn/rules/16766350)
on 2026-08-01. The committed JSON is the reviewable source for that external
setting.

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

## Remote branch lifecycle

Remote branches are temporary review or automation state, not an archive. Keep
`main`, branches with an open pull request, and branches whose unique work has
an identified owner and current purpose. Delete a branch after its pull request
is merged or closed only after confirming that every unique commit was merged,
squashed, cherry-picked, or deliberately superseded.

The audit on 2026-08-01 removed the historical feature and release-plz branches
that met that rule. It retained `release-plz-2026-07-13T21-37-30Z` because it is
the head of held release PR #59. At the end of the audit, the only other remote
branches were `main` and the in-progress branch for issue #124.

Run the following read-only audit periodically and before deleting branches:

```bash
git fetch origin --prune
git for-each-ref --format='%(refname:short)' refs/remotes/origin \
  | sed '/^origin\/HEAD$/d; /^origin\/main$/d'
git rev-list --left-right --count origin/main...origin/<branch>
git log --oneline origin/main..origin/<branch>
git cherry origin/main origin/<branch>
```

For each listed branch, also inspect its pull request and current owner. A zero
right-hand count from `rev-list` proves the branch tip is an ancestor of
`main`. A `-` from `git cherry` identifies a patch already present with a
different commit ID, which is common after squash or cherry-pick merges. Neither
command alone proves that a branch is disposable: review any `+` commits and
the pull-request diff, and record why their work is incorporated, obsolete, or
intentionally abandoned. If that cannot be established, keep the branch.

Delete only the explicitly reviewed remote name, then fetch and repeat the
listing:

```bash
git push origin --delete <branch>
git fetch origin --prune
```

Never bulk-delete from a generated list, and never delete the head branch of an
open pull request. Local branches can outlive deleted remote branches and are
not evidence that a remote branch still exists.

### release-plz branches

release-plz uses a timestamped branch for a release pull request and continues
updating that branch while its pull request remains open. The repository's
`release-plz.toml` does not provide remote-branch retention, and the release
workflow must not delete a branch immediately after creating or updating it:
that would break both the pull request and the explicitly dispatched checks.

Leave the branch for PR #59 in place for the duration of the release hold. When
a release pull request is merged, confirm the release and tag succeeded and
then delete its head branch. When one is closed without merging, first decide
whether its release commit or changelog contains work to preserve; delete the
branch only after that decision is recorded. Include `release-plz-*` branches
in the periodic audit above so interrupted or closed release attempts do not
accumulate indefinitely.
