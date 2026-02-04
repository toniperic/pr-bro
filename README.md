# PR Bro

[![CI](https://img.shields.io/github/actions/workflow/status/toniperic/pr-bro/ci.yml?branch=master)](https://github.com/toniperic/pr-bro/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/toniperic/pr-bro)](https://github.com/toniperic/pr-bro/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

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

PR Bro's Homebrew tap is hosted in a private GitHub repository. You need a GitHub token with `repo` scope (that has access to `toniperic/pr-bro` repo) to install.

```bash
export HOMEBREW_GITHUB_API_TOKEN=ghp_your_token_here
brew tap toniperic/tap
brew install pr-bro
```

Create a token at https://github.com/settings/tokens.

To upgrade:

```bash
brew upgrade pr-bro
```

### From Source

Clone and build with Cargo:

```bash
git clone git@github.com:toniperic/pr-bro.git
cd pr-bro
cargo install --path .
```

Requires Rust toolchain. Install from [rustup.rs](https://rustup.rs).

### Binary Download

Download pre-built binaries from the [GitHub Releases](https://github.com/toniperic/pr-bro/releases) page.

Available archives:
- macOS Intel: `pr-bro-<version>-x86_64-apple-darwin.tar.gz`
- macOS Apple Silicon: `pr-bro-<version>-aarch64-apple-darwin.tar.gz`
- Linux: `pr-bro-<version>-x86_64-unknown-linux-gnu.tar.gz`

Extract and move to your PATH:

```bash
tar -xzf pr-bro-<version>-<platform>.tar.gz
mv pr-bro /usr/local/bin/
```

## Quick Usage

Set your GitHub token and run:

```bash
export PR_BRO_GH_TOKEN=ghp_your_token_here
pr-bro
```

Create `~/.config/pr-bro/config.yaml` with at least one query:

```yaml
queries:
  - name: my-reviews
    query: "is:pr review-requested:@me"
```

Use `pr-bro --help` for all command-line options. Press `?` in the TUI for keyboard shortcuts.

## Features

**Weighted scoring** calculates a single priority number for each PR based on age, approval count, size, labels, and whether you've reviewed it before, all based on your preferences/configuration. Each parameter can be used to boost or penalize PRs score in any way you see fit.

**Interactive TUI** shows all PRs sorted by score. Navigate with arrow keys or vim bindings. Press `d` to see the score breakdown for any PR. Press `r` to refresh.

**Multiple queries** let you track different PR sets. Each query can override global scoring rules. First-match-wins when a PR appears in multiple queries.

**Snooze PRs** to hide them temporarily. Press `s` to snooze for a custom duration or indefinitely. Snoozed PRs live in a separate tab and don't clutter your main list.

**Score breakdown** shows exactly how a PR's score was calculated. See which factors contributed most. Press `d` on any PR to open the detail view.

**ETag-based HTTP caching** reduces GitHub API calls. Auto-refresh only fetches if data changed on the server. Manual refresh bypasses in-memory cache.

## Configuration

Configuration file location: `~/.config/pr-bro/config.yaml`

Minimal example with one query:

```yaml
queries:
  - name: my-reviews
    query: "is:pr review-requested:@me"
```

For full scoring options, per-query overrides, effect syntax, and validation details, see [Configuration Reference](docs/configuration.md).

For cache location and behavior, see [Caching](docs/caching.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and commit message format.

## License

[MIT](LICENSE)
