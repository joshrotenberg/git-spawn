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

### 2026-08-01 audit record

The comparison base was `main` at
`2f4148e8da630f98013bbc50dd7467dca191d2e7`. The following table records every
deleted ref and its pre-deletion tip. "Ancestor" means `git merge-base
--is-ancestor <tip> main` succeeded; the named merge commit and pull request
provide the durable incorporation evidence. A recorded tip can also be used to
reconstruct a deleted ref while the object remains available.

| Deleted branch | Tip | Pull request / disposition |
| --- | --- | --- |
| `feat/porcelain` | `f87d70dcd9c3c90a428cff9bc34771f9a16d30e6` | Ancestor; merged by PR #1 (`0456650`). |
| `feat/ci` | `821bc20174fe01468761633dd381ea452385292f` | Ancestor; merged by PR #2 (`992ef61`). |
| `feat/plumbing-and-docs` | `bde0d19a9a62c517b79db599e1b6b0dbb808a268` | Ancestor; merged by PR #3 (`31f4cfe`). |
| `feat/advanced` | `bab562422228cdb37fc0acfe62e80f3378820a13` | Ancestor; merged by PR #4 (`8288d7d`). |
| `docs/readme` | `5199d07580bd8c253e773582935828cd9e6cddc3` | Ancestor; merged by PR #5 (`89c29be`). |
| `feat/examples-and-plumbing` | `4d840f5e04078c8714cffc303d41beb4edf7e4cb` | Ancestor; merged by PR #6 (`017964c`). |
| `feat/workflow-modules` | `8b80d91c4ae1ee1fd54ddac43f19cf7c7d0f94bd` | Ancestor; merged by PR #8 (`af752c9`). |
| `feat/workflow-modules-pt2` | `dee8dc615f8dccf133bceeff278051ebe96cd740` | Ancestor; merged by PR #9 (`7d611c1`). |
| `arsenalotto/issue-128-executor-support-piped-stdin-bytes` | `5d2afc7ebefdbb1faab14a959f6df782eb71b63c` | PR #131 was closed as superseded by issue #132 and merged PR #133 (`3893cd6`). The later implementation preserves the intended stdin-byte support on `main`. |
| `release-plz-2026-04-14T04-18-24Z` | `14cef5757ad9aaa2c084f0c0eddaf49119c0d22a` | Abandoned release-plz attempt; its only unique commit was an obsolete `v0.1.1` release bump. Superseded by the published `v0.1.1` through `v0.2.1` history and active release PR #59 for `v0.3.0`. |
| `release-plz-2026-04-14T04-31-20Z` | `de3c06d08206b8bec85ee1f7e7795660528c7407` | Same abandoned `v0.1.1` disposition; no open PR used this branch. |
| `release-plz-2026-04-14T04-41-55Z` | `af656ede17307212509b5cb51983b3c1754fa46f` | Same abandoned `v0.1.1` disposition; no open PR used this branch. |
| `release-plz-2026-04-14T04-47-46Z` | `a9b56782875ef6798530dd8fe04c36e20c5c1a3d` | Same abandoned `v0.1.1` disposition; no open PR used this branch. |
| `release-plz-2026-04-14T13-43-42Z` | `75a39d5a657a4ba6942912305a51ddbcfc6117c4` | Same abandoned `v0.1.1` disposition; no open PR used this branch. |
| `release-plz-2026-05-22T02-06-36Z` | `95ef7fa406539803b136f27f1e363ef8858c80e8` | Same abandoned `v0.1.1` disposition; no open PR used this branch. |
| `release-plz-2026-05-22T03-39-08Z` | `cf3b0ea5eee28705a1df9a4ffe880b0a61e646e2` | Same abandoned `v0.1.1` disposition; no open PR used this branch. |

The release-plz tips were each 82--95 commits behind the comparison base. Their
unique changes only proposed an old version/changelog bump; later release
commits `111a980` (`v0.1.1`), `d70183e` (the `v0.2.0` changelog), and `ff25b1f`
(`v0.2.1`) superseded that generated state. The audit therefore deliberately
abandoned those unique bot commits rather than treating them as merged work.
The retained PR #59 branch was separately verified as the sole open release PR.

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

Record the exact remote tip before auditing it. Delete only the explicitly
reviewed remote name, with an expected-SHA lease that makes the deletion fail
if the branch moved after the audit, then fetch and repeat the listing:

```bash
git rev-parse origin/<branch> # record this as <reviewed-tip>
# Perform the audit above against that exact tip before continuing.
git push --force-with-lease=refs/heads/<branch>:<reviewed-tip> \
  origin :refs/heads/<branch>
git fetch origin --prune
```

Treat a lease failure as evidence that the branch changed: fetch it and repeat
the complete audit against the new tip instead of retrying with a broader force
or an unqualified deletion. Never bulk-delete from a generated list, and never
delete the head branch of an open pull request. Local branches can outlive
deleted remote branches and are not evidence that a remote branch still
exists.

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
