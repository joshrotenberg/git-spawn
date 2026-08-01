# Repository automation setup

## Release pull-request token

The `Release` workflow must update release-plz pull requests with a credential
other than the workflow-provided `GITHUB_TOKEN`. GitHub deliberately prevents
most events created by `GITHUB_TOKEN` from starting another workflow, so those
updates otherwise receive no CI checks.

Create the Actions secret `RELEASE_PLZ_TOKEN` using a fine-grained personal
access token with access only to this repository and these permissions:

- Contents: read and write
- Pull requests: read and write

The credential's owner must be allowed to push to the release-plz branch.
Rotate the secret according to the repository's normal credential policy. A
GitHub App is also suitable, but its short-lived installation token must be
generated during each workflow run rather than stored as this secret. Do not
replace the credential with `GITHUB_TOKEN`: the release PR will still update,
but its `synchronize` event will not start CI.

## Protecting `main`

Import [rulesets/main.json](rulesets/main.json) as a repository ruleset (or
create an equivalent ruleset under **Settings > Rules > Rulesets**). Keep it
active and require both stable check names:

- `Required checks` is the rollup for every matrix and non-matrix job in
  `ci.yml`.
- `Conventional PR title` protects the squash-merge commit format used by
  release-plz.

The ruleset requires an up-to-date pull request, resolved review conversations,
and squash merge; it also blocks deletion and force-pushes. Repository
administrators have the deliberate emergency bypass (`RepositoryRole` id 6).
Use that bypass only for incident recovery, record the reason, and immediately
restore the branch to a state that passes both checks.

Do not merge the current release pull request merely to test this setup.
Publishing remains held until the planned implementation, API hardening, and
documentation work is complete.

## Safe verification

1. Install `RELEASE_PLZ_TOKEN`, then wait for the next non-release merge to
   `main` to run `Release` normally.
2. On release PR #59 (or a disposable PR produced by release-plz), confirm that
   the bot's new commit produces a `synchronize` event and starts `CI` and `PR
   Title` without manually rerunning either workflow.
3. Confirm `Required checks` stays pending until all CI jobs finish and then
   succeeds only when all of them succeed.
4. Confirm GitHub reports the PR behind `main` as blocked and requires it to be
   updated, and that a failed required check blocks merge.
5. Close the disposable PR if one was used. Do not merge the release PR and do
   not run a publish operation as part of verification.
