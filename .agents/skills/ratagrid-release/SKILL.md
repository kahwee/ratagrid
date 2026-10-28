---
name: ratagrid-release
description: Prepare, publish, resume or verify a Ratagrid release with local checks, clean package verification, exact-source GitHub CI, annotated tags, crates.io and documentation status.
---

# Ratagrid release

Use [the release runbook](../../../docs/RELEASING.md) for commands. Read
`AGENTS.md`, `CONTRIBUTING.md`, package metadata and current workflows first.

## 1. Establish the release

- Honor the user's requested destinations and existing authorization. A GitHub
  source release and a crates.io upload are separate outcomes. Do not repeat
  approvals already given or add Docker/other registries to the scope.
- Inspect the working tree, remote main, tags, GitHub releases and registry versions.
  Preserve user edits and concurrent commits. Never reuse a version for different
  source or move a release tag, including a source-only tag absent from crates.io.
- Use an explicit version. Otherwise retain a prepared unpublished candidate, or
  choose the next unused version: patch for compatible fixes, minor for new public
  APIs or breaking pre-1.0 changes. State the choice; clarify only real conflicts.
- Check authentication early using supported CLI/provider readiness, without
  revealing tokens. Do not infer missing Cargo auth from an absent environment
  variable alone. Do not create credentials, access grants or trusted-publisher
  configuration. Continue independent work if a destination is blocked.

## 2. Prepare once

- Update Cargo.toml and only Ratagrid's Cargo.lock version; avoid dependency churn.
- Update CHANGELOG.md, README installation instructions, guides and the Pages
  template. Separate unreleased main features from tagged and registry versions.
  Regenerate bundled guides and Pages; keep MIT attribution and synthetic fixtures.
- Run `python3 scripts/release_check.py check` once after the final implementation
  changes. It executes the host's local gates without uploading or tagging.
  Fix failures; repeat affected checks after subsequent edits.
- Finish authorized commits/pushes. Record the full release SHA and require CI
  success on that SHA across Linux, macOS and Windows. Earlier green runs do not
  count. Keep existing branch rules and do not dispatch benchmarks just to release.

## 3. Freeze and package

- Use a clean checkout of the tested SHA. If main moves, inspect the difference
  and explicitly retain the tested source or fully verify the newer source.
- Run `python3 scripts/release_check.py package`. This performs one Cargo publish
  dry run, checks archive contents against Git, validates VCS/metadata/license/
  dependencies and writes a SHA-256 report. It never uploads or creates tags.
- Review package contents for private data. No `--allow-dirty`, `--no-verify`,
  source edits after verification, or silent version substitutions.

## 4. Publish only the requested destinations

- Re-read every destination before mutation. Confirm the external crates.io
  binding matches owner `kahwee`, repository `ratagrid`, workflow `publish-crate.yml`,
  blank environment. Honor the release owner's coordination gate before tagging;
  do not configure credentials or trust as a shortcut.
- A new stable user-pushed tag starts the approved publisher after metadata,
  exact-source main CI and clean package checks. Create that annotated tag only
  at the recorded SHA, without force. Never duplicate the OIDC upload manually.
- Use an identifying User-Agent for registry reads. After an uncertain upload,
  query before retrying. Verify the downloaded checksum and VCS SHA against the
  prepared report, then check the exact docs.rs version separately.
- Create the requested GitHub source release for the verified tag with truthful
  notes and read it back. Reuse matching existing tags/releases; resume only
  missing steps. Keep source-tag installation while registry publication is pending.
- Let Pages deploy normally; verify its SHA and live content separately. Keep
  the source-release proposal inactive and preserve existing permission scope.

## 5. Report and resume

Report the version, full source SHA, verified GitHub/registry/docs URLs, Pages
status, checks, checksum and any pending destinations. Preserve completed work;
never delete/yank a release to disguise partial publication. On resumption,
re-establish the recorded version/source and current destination states first.

When changing this skill, run the skill-creator `quick_validate.py` validator and
check its relative links and `agents/openai.yaml` metadata.
