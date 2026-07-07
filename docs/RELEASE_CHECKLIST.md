# Release Checklist

Use this checklist when preparing a new release of the Axiom framework.

## Pre-Release

- [ ] `cargo check --workspace` passes with no errors
- [ ] `cargo test --workspace` passes with 0 failures
- [ ] `cargo clippy --workspace` passes with no warnings (or only allowed warnings)
- [ ] `cargo doc --workspace --no-deps` builds successfully
- [ ] All documentation in `docs/` is up to date
- [ ] `CHANGELOG.md` is updated with the new version entry
- [ ] `HANDOVER.md` reflects the current migration/completion state
- [ ] Version numbers are consistent across all `Cargo.toml` files

## Version Bump

- [ ] Update `Cargo.toml` workspace version
- [ ] Update individual crate versions if needed
- [ ] Update `CHANGELOG.md` date and version links

## Testing

- [ ] Unit tests pass: `cargo test --workspace`
- [ ] Integration tests pass: `cargo test --workspace --tests`
- [ ] Doc tests pass: `cargo test --workspace --doc`
- [ ] Release build compiles: `cargo build --workspace --release`

## Security

- [ ] `cargo audit` passes with no vulnerabilities
- [ ] `cargo deny check` passes
- [ ] No secrets or credentials in the repository

## Documentation

- [ ] `README.md` is up to date
- [ ] `docs/API_BOUNDARY.md` reflects any breaking changes
- [ ] `docs/MIGRATION.md` is updated if there are migration steps
- [ ] `docs/VERSIONING.md` deprecation policy is followed

## Git

- [ ] All changes are committed
- [ ] Commit messages follow conventional commits format
- [ ] Tag is created: `git tag vX.Y.Z`
- [ ] Tag is pushed: `git push origin vX.Y.Z`

## Post-Release

- [ ] GitHub release is published with `CHANGELOG.md` notes
- [ ] Crates.io publish: `cargo publish --workspace` (for each crate in order)
- [ ] Announce release on appropriate channels
