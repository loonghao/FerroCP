# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


### Added
- Initial project structure
- Basic repository setup

## [1.0.0](https://github.com/loonghao/FerroCP/compare/v0.4.1...v1.0.0) (2026-09-25)


### ⚠ BREAKING CHANGES

* **copy:** CopyOptions::preserve_metadata is split into preserve_timestamps and preserve_permissions; CopyStats gains symlinks_created; CopyMode::Mirror now returns an error instead of behaving like All; Python CopyOptions.overwrite default changes from "prompt" to "always" (runtime behaviour is unchanged, the old default advertised protection that did not exist) and code passing overwrite="prompt" must now also pass overwrite_callback.

### Features

* **copy:** define and enforce the copy semantics contract ([f74b110](https://github.com/loonghao/FerroCP/commit/f74b110ffdb05fa1cdefd257b0622f355b114c4e))


### Bug Fixes

* **ci:** clear the clippy and pyo3 test failures blocking the merge gate ([#106](https://github.com/loonghao/FerroCP/issues/106)) ([7fc6c2c](https://github.com/loonghao/FerroCP/commit/7fc6c2ce45dfbb4683336ac0bf2248a3a66035d8))
* **ci:** supply a macOS SDK so darwin release targets link ([#98](https://github.com/loonghao/FerroCP/issues/98)) ([b0b6009](https://github.com/loonghao/FerroCP/commit/b0b6009b59023bf391c38de486f0f0ce11aa0b95))
* **ci:** unblock the GoReleaser release pipeline ([ada3be0](https://github.com/loonghao/FerroCP/commit/ada3be0e0712f20430b204cf67b75747eb1e06b1))
* **cli:** derive the result message from the performance rating ([#110](https://github.com/loonghao/FerroCP/issues/110)) ([4373b00](https://github.com/loonghao/FerroCP/commit/4373b000756343ae5238b0593f482bdb0d99a29d))
* **deps:** pin bincode to 2.x and commit Cargo.lock ([#92](https://github.com/loonghao/FerroCP/issues/92)) ([7767c08](https://github.com/loonghao/FerroCP/commit/7767c0809fb316a698906f845b791b767119027f))
* **python:** build the ferrocp-python bindings and add them to the workspace ([f66db9c](https://github.com/loonghao/FerroCP/commit/f66db9c3139bb43fccccb83662fc50189885f93e))
* **python:** make move() fail fast instead of losing data ([#108](https://github.com/loonghao/FerroCP/issues/108)) ([6fe6b97](https://github.com/loonghao/FerroCP/commit/6fe6b97310ea201eb15ee75e940106ffcdfc01fa))
* **zerocopy:** build the macOS backend against current libc ([27e99be](https://github.com/loonghao/FerroCP/commit/27e99beda5034fd74a43603552a08cf214c5dcca))

## v0.4.1 (2026-04-11)

### Fix

- **deps**: update rust crate nix to 0.31

## v0.4.0 (2025-06-14)

### Feat

- optimize cross-compilation with cargo-zigbuild

## v0.3.3 (2025-06-14)

### Fix

- clean up GoReleaser configuration and remove outdated scripts

## v0.3.2 (2025-06-13)

### Fix

- **deps**: update rust crate pyo3-async-runtimes to 0.25

## v0.3.1 (2025-06-13)

### Fix

- **deps**: update rust crate bincode to v2

## v0.3.0 (2025-06-13)

### Feat

- disable Python bindings and add C-ABI interface for future language bindings
- add JSON output support for automated performance testing
- migrate CI to GoReleaser with PGO optimization
- add GoReleaser local testing configuration
- **ci**: add CI configuration validation scripts
- improve CI and development workflow
- improve cross-platform builds with cross tool
- simplify CI through GoReleaser integration
- update GoReleaser to use PyPI Trusted Publishing and add Rust crates.io support
- add PyPI publishing support to GoReleaser and reorganize documentation
- add GoReleaser configuration for automated releases
- optimize CI workflows and fix configuration issues
- add comprehensive performance benchmarking and web visualization
- enhance Rust toolchain and fix code warnings
- implement comprehensive ferrocp-engine with task management
- implement comprehensive ferrocp-config and ferrocp-io crates
- implement ferrocp-types foundation and functional CLI tool
- implement Windows device detection and ReFS CoW support
- complete project rename from py-eacopy to ferrocp
- add advanced logging control and skip existing files functionality
- separate CLI and Python bindings to eliminate python.dll dependency
- optimize dependency management and add PGO build support
- complete Rust native implementation with comprehensive performance monitoring
- implement Python bindings for EACopy
- add local build and test support with nox and cibuildwheel

### Fix

- synchronize version numbers and update CI configuration
- update goreleaser configuration to v2 format
- resolve CI compilation issues and improve code style
- resolve Linux ARM64 cross-compilation linking issues
- **ci**: enhance ARM64 cross-compilation with robust dependency handling
- **ci**: resolve ARM64 cross-compilation apt source issues
- **ci**: resolve Ubuntu ARM64 apt sources configuration issues
- **ci**: resolve Linux ARM64 cross-compilation and VFX Platform Summary issues
- resolve Linux CI type error in zerocopy tests
- properly handle FerroCP async API in CodSpeed benchmarks
- resolve CodSpeed benchmark async API conflicts
- resolve pytest-benchmark plugin detection and compiler issues
- improve CLI async handling and PGO build robustness
- resolve CLI async operation conflicts in test environments
- resolve blake3 assembly compatibility issues causing exit code 157
- resolve CLI async operation and blake3 compilation issues
- resolve blake3 assembly compilation and benchmark issues
- resolve Python API compatibility and Rust test issues
- resolve CI build and benchmark issues
- add missing documentation for platform-specific zero-copy features
- resolve compilation errors and test failures
- resolve uv cache warnings in GitHub Actions
- resolve CI build failures and security vulnerabilities
- enhance CI build environment and resolve linker issues
- disable clippy and fix thiserror import issues
- optimize maturin-action with complete clang toolchain
- resolve GoReleaser build issues with linker configuration
- add build tools and environment setup for Python extension compilation
- resolve cross-compilation and linking issues in GoReleaser build
- correct maturin configuration for Python package build
- clean up dead code warnings and add missing documentation
- resolve compilation errors in device detection and memory mapping
- resolve unused variables and compilation warnings
- resolve pyo3 dependency version conflicts
- enhance macOS ring compilation fixes with additional environment variables
- resolve macOS ring compilation issues in CI/CD
- resolve Cargo.toml syntax errors and apply code formatting
- update compression example to use correct CompressionConfig fields
- resolve remaining lint issues in CLI module
- update Python version requirements from 3.8 to 3.9
- resolve compilation errors in Rust modules

### Refactor

- optimize test and mock code for better maintainability
- clean up redundant CI configurations and simplify workflows
- simplify CLI using click and update project configuration

## v0.0.0 (2025-05-07)
