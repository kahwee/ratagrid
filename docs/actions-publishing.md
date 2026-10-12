# Actions publishing

`python3 scripts/check-release.py v<version>` verifies the package version, tag
and root Cargo.lock entry without building or publishing.
`python3 scripts/test_check_release.py` covers version mismatch regressions.
Keep the repository release skill and existing required CI as the authority.

## Approval proposal

[The complete crates.io workflow](../.github/release-proposals/publish-crate.yml)
is outside `.github/workflows` and cannot run. Activation requires approval to
move it to `.github/workflows/publish-crate.yml`, grant its publish job
`contents: read`, `actions: read`, `id-token: write`, and bind crate `ratagrid` to:

- owner `kahwee`, repository `ratagrid`
- GitHub Actions workflow filename `publish-crate.yml`
- environment blank

Use [crates.io trusted publishing](https://crates.io/docs/trusted-publishing) and
the [official temporary authentication action](https://github.com/rust-lang/crates-io-auth-action).
It checks the human-pushed stable version tag and locked metadata, requires exact
source CI, verifies packaging, then authenticates and publishes. GitHub publication
and registry publication remain separate. No long-lived Cargo credential is added.
Existing versions cannot be overwritten: an already published version requires
read-back source/checksum reconciliation rather than automatically declaring success.

No registry trust or workflow access was enabled by this change. Current local
Cargo credential file existence does not validate publishing authentication.
If the crate does not yet exist, first publication/name ownership is a separate
approved owner action before a trusted-publisher binding can be established.

[The source-release workflow](../.github/release-proposals/release-source.yml) is
also inactive. Approval must cover moving it to `.github/workflows/release-source.yml`
and granting its job `contents: write` and `actions: read`. It creates a GitHub
source release for a matching existing tag after exact-source CI. It never moves
a tag; an existing release makes creation fail safely and requires read-back.

Crate publication is triggered directly by human-pushed `v*` tags; no `release: published` cascade from `GITHUB_TOKEN` is assumed. It runs independently of GitHub source/binary publication. Existing immutable historical tags without these new guard files need a separately reviewed recovery procedure; this proposal does not move or republish them.

CI runs the actual metadata CLI against the checked-out manifest/lock and rejects a mismatched tag. The publisher proposal remains unexecuted: packaging dry-run, registry exchange and publication are not proven by guard tests.
