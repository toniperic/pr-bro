# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/toniperic/pr-bro/compare/v0.3.4...v0.4.0) - 2026-02-11

### Added

- add light and dark theme support with auto-detection

### Other

- disable duplicate GitHub auto-generated release notes
- replace theme constants with ThemeColors struct

## [0.3.4](https://github.com/toniperic/pr-bro/compare/v0.3.3...v0.3.4) - 2026-02-10

### Fixed

- add inner padding to score breakdown modal ([#66](https://github.com/toniperic/pr-bro/pull/66))

### Other

- *(deps)* bump serde-saphyr from 0.0.17 to 0.0.18
- *(deps)* bump webbrowser from 1.0.6 to 1.1.0
- *(deps)* bump reqwest from 0.13.1 to 0.13.2
- default query to review-requested:@me review:required is:open

## [0.3.3](https://github.com/toniperic/pr-bro/compare/v0.3.2...v0.3.3) - 2026-02-08

### Fixed

- prevent caching of truncated GitHub API responses
- buffer stderr during TUI mode to prevent display corruption

## [0.3.2](https://github.com/toniperic/pr-bro/compare/v0.3.1...v0.3.2) - 2026-02-08

### Fixed

- suppress enrichment warnings that corrupt TUI display ([#57](https://github.com/toniperic/pr-bro/pull/57))

## [0.3.1](https://github.com/toniperic/pr-bro/compare/v0.3.0...v0.3.1) - 2026-02-08

### Other

- auto-evict stale cache entries on startup ([#55](https://github.com/toniperic/pr-bro/pull/55))

## [0.3.0](https://github.com/toniperic/pr-bro/compare/v0.2.4...v0.3.0) - 2026-02-06

### Added

- add 20s timeout to refresh fetch to prevent indefinite hangs

### Fixed

- lighten muted text color for better readability

### Other

- remove downloads badge from README

## [0.2.4](https://github.com/toniperic/pr-bro/compare/v0.2.3...v0.2.4) - 2026-02-05

### Fixed

- update rpassword API call for v7.x compatibility

### Other

- *(quick-025)* filter release-plz to meaningful commit types
- run cargo fmt
- *(deps)* bump rpassword from 5.0.1 to 7.4.0
- add Asciinema demo link to README

## [0.2.3](https://github.com/toniperic/pr-bro/compare/v0.2.2...v0.2.3) - 2026-02-05

### Other

- *(deps)* bump actions/upload-artifact from 4 to 6
- *(deps)* bump actions/github-script from 7 to 8
- simplify Homebrew installation instructions
- remove private download strategy
- *(deps)* bump actions/download-artifact from 4 to 7
- *(deps)* bump serde-saphyr from 0.0.16 to 0.0.17
- *(quick-021)* add shields.io badges for CI, crates.io version and downloads
- *(deps)* bump clap from 4.5.54 to 4.5.57
- *(deps)* bump anyhow from 1.0.100 to 1.0.101
- *(deps)* bump time from 0.3.46 to 0.3.47
- *(deps)* bump atomic-write-file from 0.2.3 to 0.3.0
- *(deps)* bump actions/checkout from 4 to 6
- *(deps)* bump jsonwebtoken from 10.2.0 to 10.3.0
- add Dependabot for dependency updates
- *(deps)* bump bytes from 1.11.0 to 1.11.1
- track Cargo.lock for reproducible builds
- simplify README installation sections
- pin cross to v0.2.5 in release workflow
- remove redundant release build job
- *(quick-019)* simplify README quick start and remove Configuration section
- *(tui)* rename "detail" nav hint to "breakdown"

## [0.2.2](https://github.com/toniperic/pr-bro/compare/v0.2.1...v0.2.2) - 2026-02-04

### Other

- bump minor version on feat commits in pre-stable releases ([#20](https://github.com/toniperic/pr-bro/pull/20))

## [0.2.1](https://github.com/toniperic/pr-bro/compare/v0.2.0...v0.2.1) - 2026-02-04

### Added

- change score breakdown keybind from 'd' to 'b' ([#10](https://github.com/toniperic/pr-bro/pull/10))

### Fixed

- *(ci)* use PAT for release-plz to trigger CI on release PRs ([#15](https://github.com/toniperic/pr-bro/pull/15))
- *(ci)* disable semver-check for binary crate ([#14](https://github.com/toniperic/pr-bro/pull/14))
- update docs references from 'd' to 'b' for score breakdown
- update footer hint from 'd' to 'b' for score breakdown ([#12](https://github.com/toniperic/pr-bro/pull/12))
