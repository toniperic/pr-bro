# Versioning

PR Bro uses [Semantic Versioning 2.0](https://semver.org/) with automated version bumping via [release-plz](https://release-plz.dev/).

## Version Bump Rules

Commits trigger version bumps based on type:

| Commit Type | Example | Version Bump |
|-------------|---------|--------------|
| `feat` | `feat(tui): add score breakdown view` | 0.1.0 -> 0.2.0 (MINOR) |
| `fix` | `fix(cache): handle network timeout` | 0.1.0 -> 0.1.1 (PATCH) |
| `perf` | `perf(scoring): optimize label matching` | 0.1.0 -> 0.1.1 (PATCH) |
| `feat!` / `BREAKING CHANGE:` | `feat(config)!: change YAML schema` | 0.1.0 -> 1.0.0 (MAJOR) |
| `docs`, `style`, `refactor`, `test`, `build`, `ci`, `chore` | Any | No bump |

## Pre-1.0 Semantics

During 0.x releases (initial development), minor version bumps may include breaking changes per the SemVer 2.0 spec.

### Breaking Changes for PR Bro

These changes warrant a major version bump (or minor bump during 0.x):

- CLI flag renames or removals
- Config schema changes requiring user edits to existing configs
- Output format changes that break scripting or automation
- Removal of features

### Non-Breaking Changes

These are safe additive changes:

- Adding new features (as long as existing features work unchanged)
- Bug fixes that change behavior to match documentation
- Internal refactoring with no user-visible changes
- Performance improvements

Normal SemVer rules apply after 1.0.0 — breaking changes always trigger a major bump.

## Automated Release Process

The release workflow runs automatically:

1. Merge a PR to `master`
2. release-plz analyzes commits since last release using conventional commit format
3. Calculates version bump based on commit types
4. Opens a release PR with updated Cargo.toml version
5. Merge the release PR
6. release-plz creates a git tag (e.g., `v0.2.0`)
7. Tag triggers release workflow — builds binaries for macOS (Intel + Apple Silicon) and Linux (x64)
8. GitHub Release created with binaries and auto-generated release notes
9. Homebrew tap (toniperic/homebrew-tap) automatically updated with new formula

## Pre-Release Versions

release-plz supports pre-release versioning for gradual rollouts:

- `0.2.0-alpha` — early testing, unstable API, frequent breaking changes expected
- `0.2.0-beta` — feature-complete, API stabilizing, bug fixes only
- `0.2.0-rc.1` — release candidate, production-ready testing, critical fixes only

Pre-releases have lower precedence than stable versions per SemVer (`0.2.0-alpha < 0.2.0-beta < 0.2.0-rc.1 < 0.2.0`).
