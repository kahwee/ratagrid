# Tag-only publishing

The approved [crate publisher](../.github/workflows/publish-crate.yml) runs only
for future stable `v*` tags pushed by user accounts. Branch pushes, pull requests
and manual dispatches do not publish.

Before OIDC authentication, it validates tag/manifest/root-lock agreement,
requires successful exact-source main CI across all three OS jobs, and verifies
a clean Cargo package against Git, its inventory, license and VCS metadata.
The official action then obtains a short-lived credential for one locked upload.

The job has only `contents: read`, `actions: read`, `id-token: write`, with no
GitHub environment or stored Cargo secret. Registry trust must separately match
owner `kahwee`, repository `ratagrid`, workflow `publish-crate.yml`, blank environment.
Workflow activation alone does not establish that external trust or test an upload.

Version 0.5.0 is already published from its immutable tag. Version 0.5.1 is prepared
maintenance; its tag/upload remains on hold until the release owner confirms the
binding and releases the coordination gate. Never repush old tags or duplicate
an uncertain upload; inspect registry state first.

The [source-release proposal](../.github/release-proposals/release-source.yml)
remains inactive. GitHub releases, registry uploads, docs.rs and Pages are verified
separately. Follow the [release runbook](RELEASING.md) for exact-source receipts,
checksums and recovery; [Actions maintenance](../.github/ACTIONS.md) records the pins.
