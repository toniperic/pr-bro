# Contributing to PR Bro

## Quick Start

Get up and running:

```bash
git clone git@github.com:toniperic/pr-bro.git
cd pr-bro
cargo build
cargo test
```

Run with a GitHub token:

```bash
export PR_BRO_GH_TOKEN=ghp_your_token_here
cargo run
```

## GitHub Token Setup

Create a personal access token at https://github.com/settings/tokens

Required scopes:
- `repo` — for private repositories
- `public_repo` — for public repositories only (if you don't need private repo access)

The app checks for the `PR_BRO_GH_TOKEN` environment variable first. If not set, you'll be prompted interactively at startup.

## Code Style

Run before committing:

```bash
cargo fmt
cargo clippy
```

CI enforces both. Clippy runs with `-D warnings` — all warnings are treated as errors.

## Testing

Run the test suite:

```bash
cargo test
```

## Commit Message Format

All commits must follow [Conventional Commits 1.0.0](https://www.conventionalcommits.org/).

Format: `<type>[optional scope]: <description>`

### Commit Types

| Type | Description | Version Bump |
|------|-------------|--------------|
| `feat` | New feature | MINOR |
| `fix` | Bug fix | PATCH |
| `docs` | Documentation only | none |
| `style` | Code style (formatting, whitespace) | none |
| `refactor` | Code refactoring (no functional change) | none |
| `perf` | Performance improvement | PATCH |
| `test` | Add or update tests | none |
| `build` | Build system changes | none |
| `ci` | CI configuration changes | none |
| `chore` | Other changes (dependencies, etc.) | none |

### Breaking Changes

Add `!` after the type or `BREAKING CHANGE:` in the footer to trigger a MAJOR version bump:

- `feat(config)!: change YAML schema format`
- `fix(api)!: remove deprecated endpoint`

### Scope Examples

Scope is optional but recommended:

- `feat(scoring): add label-based scoring factor`
- `fix(tui): prevent panic on empty PR list`
- `docs: update installation instructions` (no scope needed)

### Concrete Examples

```
feat(scoring): add label-based scoring factor
fix(tui): prevent panic on empty PR list
docs: update installation instructions
feat(api)!: change config schema format
```

### CI Validation

PR titles are validated in CI against this format. Non-compliant PRs are blocked until the title is fixed.

## Release Process

See [VERSIONING.md](VERSIONING.md) for how commits map to releases.
