# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.3](https://github.com/toniperic/pr-bro/compare/v0.2.2...v0.2.3) - 2026-02-05

### Other

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
