# Ratagrid

- Read `CONTRIBUTING.md`; use the toolchain in `rust-toolchain.toml` and locked Cargo commands.
- Keep ordering, filtering and selection in `GridModel`; terminal interaction belongs in `Grid`. Applications own I/O, focus and persistence.
- Preserve stable row identity, insertion-order sort ties, Unicode handling and equivalent mouse/keyboard behavior. Add regression coverage for behavior changes.
- Update `CHANGELOG.md` under Unreleased and the relevant guides. Regenerate bundled rustdoc and Pages from their sources; never edit generated files directly.
- Run `python3 scripts/release_check.py check` for the complete local checks. Follow `CONTRIBUTING.md` for focused development checks.
- Follow `.agents/skills/ratagrid-release/SKILL.md` for releases and `docs/RELEASING.md` for commands. Publish only requested destinations, from the exact commit that passed CI; never move a release tag.
- Preserve user edits and concurrent commits. Keep credentials out of files, logs and chat. Keep Action references current using `.github/ACTIONS.md`.
