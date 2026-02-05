# PR Bro

[![CI](https://github.com/toniperic/pr-bro/actions/workflows/ci.yml/badge.svg)](https://github.com/toniperic/pr-bro/actions/workflows/ci.yml) [![Crates.io](https://img.shields.io/crates/v/pr-bro.svg)](https://crates.io/crates/pr-bro) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[![Demo](https://asciinema.org/a/780716.svg)](https://asciinema.org/a/780716)

Know which PR to review next. PR Bro ranks pull requests by weighted scoring across your GitHub queries, so you always start with the most important review.

## Requirements

- **GitHub Personal Access Token**: 
  - `repo` scope for private repos
  - `public_repo` for public only
- **Platforms**: 
  - macOS (Intel + Apple Silicon)
  - Linux (x64)

## Installation

### Homebrew (macOS)

```bash
brew tap toniperic/tap
brew install pr-bro
```

To upgrade:

```bash
brew upgrade pr-bro
```

### Cargo

```bash
cargo install pr-bro
```

Requires Rust toolchain. Install from [rustup.rs](https://rustup.rs).

### Binary Download

Download pre-built binaries from the [GitHub Releases](https://github.com/toniperic/pr-bro/releases) page. Extract and move to your PATH:

```bash
tar -xzf pr-bro-<version>-<platform>.tar.gz
mv pr-bro /usr/local/bin/
```

## Quick Usage

```bash
pr-bro
```

On first run, PR Bro will prompt you with a series of questions to set up your configuration. It will also ask for your GitHub token interactively. To skip the token prompt, set the `PR_BRO_GH_TOKEN` environment variable.

For the full list of configuration options, see the [Configuration Reference](docs/configuration.md).

Use `pr-bro --help` for all command-line options. Press `?` in the TUI for keyboard shortcuts.

## Features

**Weighted scoring** calculates a single priority number for each PR based on age, approval count, size, labels, and whether you've reviewed it before, all based on your preferences/configuration. Each parameter can be used to boost or penalize PRs score in any way you see fit.

**Interactive TUI** shows all PRs sorted by score. Navigate with arrow keys or vim bindings. Press `b` to see the score breakdown for any PR. Press `r` to refresh.

**Multiple queries** let you track different PR sets. Each query can override global scoring rules. First-match-wins when a PR appears in multiple queries.

**Snooze PRs** to hide them temporarily. Press `s` to snooze for a custom duration or indefinitely. Snoozed PRs live in a separate tab and don't clutter your main list.

**Score breakdown** shows exactly how a PR's score was calculated. See which factors contributed most. Press `b` on any PR to open the detail view.

**ETag-based HTTP caching** reduces GitHub API calls. Auto-refresh only fetches if data changed on the server. Manual refresh bypasses in-memory cache.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and commit message format.

## License

[MIT](LICENSE)
