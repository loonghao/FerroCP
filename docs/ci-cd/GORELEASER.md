# GoReleaser Configuration for FerroCP

This document explains how FerroCP uses GoReleaser to automate releases and simplify the build process.

## Overview

GoReleaser is a release automation tool that can handle:
- Cross-platform binary builds
- Archive creation (tar.gz, zip)
- Checksum generation
- GitHub releases
- Docker images
- Package managers (Homebrew, Scoop)
- Changelog generation

## Configuration

The GoReleaser configuration is in `.goreleaser.yml` and includes:

### Build Targets

FerroCP builds for the following platforms:
- **Linux**: x86_64, ARM64
- **macOS**: x86_64 (Intel), ARM64 (Apple Silicon)
- **Windows**: x86_64

### Build Process

1. **Before Hooks**: Install Rust targets, run tests, build Python wheels
2. **Build**: Use custom Rust build scripts for cross-compilation
3. **Archive**: Create platform-specific archives
4. **Checksum**: Generate SHA256 checksums
5. **Release**: Create GitHub release with assets
6. **Publish**: Publish to PyPI (Python) and crates.io (Rust)

### Publishing Targets

#### Python Package (PyPI)
- **Method**: Trusted Publishing (OIDC) - no tokens required
- **Artifacts**: Cross-platform wheels and source distribution
- **Platforms**: Linux, macOS, Windows (x86_64, ARM64)

#### Rust Crates (crates.io)
- **Method**: Cargo publish with registry token
- **Crates**: Library crates only (excludes CLI, Python bindings, tests)
- **Order**: Dependency-ordered publishing to handle workspace dependencies

## Setup Requirements

### PyPI Trusted Publishing Setup

1. **Go to PyPI**: Visit [https://pypi.org/manage/account/publishing/](https://pypi.org/manage/account/publishing/)
2. **Add Trusted Publisher**:
   - Publisher: GitHub
   - Owner: `loonghao`
   - Repository name: `FerroCP`
   - Workflow name: `goreleaser.yml`
   - Environment name: (leave empty)
3. **No tokens needed**: Trusted Publishing uses OIDC for secure authentication

### Rust crates.io Setup

1. **Get API Token**: Visit [https://crates.io/settings/tokens](https://crates.io/settings/tokens)
2. **Create Token**: Generate a new API token with publish permissions
3. **Add to GitHub Secrets**: Add the token as `CARGO_REGISTRY_TOKEN`

### GitHub Secrets Required

- `CARGO_REGISTRY_TOKEN`: Your crates.io API token
- No PyPI token needed (uses Trusted Publishing)

## Usage

### Local Testing

Test the GoReleaser configuration locally:

```bash
# Install GoReleaser
go install github.com/goreleaser/goreleaser@latest

# Test the configuration (dry-run)
goreleaser release --snapshot --clean

# Check the generated artifacts
ls -la dist/
```

### Release Process

Tags are **not** created by hand. [release-please](https://github.com/googleapis/release-please)
owns the version, the changelog and the tag:

1. Land conventional commits on `main`.
2. Merge the release PR that `release-please` opens (`chore(main): release x.y.z`).
3. `release-please` cuts the `vx.y.z` tag and creates the GitHub Release.
4. `release-please.yml` then invokes this workflow for that tag, which will automatically:
   - Build all platform binaries with `cargo zigbuild`
   - Create archives and checksums
   - Upload all artifacts to the release `release-please` created

Pushing a tag manually no longer starts a release. To rebuild and re-publish an
existing tag, trigger the **Release** workflow from the Actions tab and set `ref`
to that tag (for example `v0.4.1`).

### Manual Release

You can also trigger a release manually:

```bash
# Set the GitHub token
export GITHUB_TOKEN="your-github-token"

# Run GoReleaser
goreleaser release --clean
```

## Scripts

### `scripts/build-all-targets.sh`

Builds FerroCP for all supported platforms:
- Installs cross-compilation tools
- Sets up platform-specific environment
- Builds optimized binaries
- Strips debug symbols
- Tests binaries when possible

### `scripts/build-cross.sh`

Individual target builder used by the main script:
- Handles single target compilation
- Platform-specific configuration
- Binary verification

## GitHub Actions

### `.github/workflows/goreleaser.yml`

Automated release workflow that:
- Is called by `release-please.yml` with the freshly cut tag (`workflow_call`)
- Can also be started by hand (`workflow_dispatch`)
- Sets up cross-compilation environment
- Runs GoReleaser
- Supports dry-run mode

### Caller: `.github/workflows/release-please.yml`

Runs on every push to `main`. It opens or updates the release PR, and once that PR is
merged it cuts the tag and calls `goreleaser.yml`.

The call is deliberate rather than an `on: push: tags:` trigger: GitHub does not start a
new workflow run for events raised by the default `GITHUB_TOKEN`, and that token is what
creates the tag. The repository history confirms it — six tags pushed by the previous
commitizen workflow produced zero runs of this workflow.

### Workflow Dispatch

You can manually trigger the workflow with options:
- **ref**: the tag or ref to build (empty means the current ref)
- **Dry-run**: build only, without publishing

## Package Managers

### Homebrew (macOS/Linux)

> **Status: not available yet.** No Homebrew install command works today.
>
> FerroCP has never published a release asset — the only release, `v0.0.0`, ships
> zero assets — and `.goreleaser.yml` defines no `brews:` publisher. The
> `loonghao/homebrew-tap` repository also contains no `ferrocp` formula.

Once release assets are published, the intended install path is to tap
`loonghao/tap` and install the `ferrocp` formula from it. Until then, build from
source instead:

```bash
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP
uv sync --group all
uv run maturin develop --release
```

### Scoop (Windows)

> **Status: not available yet.** No Scoop install command works today.
>
> FerroCP has never published a release asset — the only release, `v0.0.0`, ships
> zero assets — and `.goreleaser.yml` defines no `scoops:` publisher. The
> `loonghao/scoop-bucket` repository also contains no `ferrocp` manifest.

Once release assets are published, the intended install path is to add the
`loonghao/scoop-bucket` bucket and install the `ferrocp` package from it. Until
then, build from source instead:

```bash
git clone https://github.com/loonghao/FerroCP.git
cd FerroCP
uv sync --group all
uv run maturin develop --release
```

## Docker Images

GoReleaser builds minimal Docker images:
```bash
# Pull the latest image
docker pull ghcr.io/loonghao/ferrocp:latest

# Run FerroCP in a container
docker run --rm ghcr.io/loonghao/ferrocp:latest --version
```

## Benefits of GoReleaser

### Simplified Release Process

**Before GoReleaser**:
- Manual cross-compilation for each platform
- Manual archive creation
- Manual checksum generation
- Manual GitHub release creation
- Manual asset uploads
- Complex CI/CD configuration

**With GoReleaser**:
- Single configuration file
- Automatic cross-compilation
- Automatic packaging and checksums
- Automatic release creation
- Integrated package manager support
- Simplified CI/CD

### Consistency

- Standardized release artifacts
- Consistent naming conventions
- Automated changelog generation
- Reproducible builds

### Integration

- GitHub integration
- Package manager integration
- Docker registry integration
- Notification systems

## Troubleshooting

### Cross-compilation Issues

If cross-compilation fails:

1. **Install required tools**:
   ```bash
   # Linux ARM64
   sudo apt-get install gcc-aarch64-linux-gnu
   
   # Windows
   sudo apt-get install gcc-mingw-w64-x86-64
   ```

2. **Check Rust targets**:
   ```bash
   rustup target list --installed
   ```

3. **Test individual targets**:
   ```bash
   cargo build --target x86_64-unknown-linux-gnu --release
   ```

### GoReleaser Issues

1. **Validate configuration**:
   ```bash
   goreleaser check
   ```

2. **Debug mode**:
   ```bash
   goreleaser release --snapshot --clean --debug
   ```

3. **Check logs**:
   ```bash
   # GitHub Actions logs
   # Local: check terminal output
   ```

## Customization

### Adding New Platforms

To add a new platform:

1. Add Rust target to `before.hooks` in `.goreleaser.yml`
2. Add new build configuration in `builds` section
3. Update `scripts/build-all-targets.sh`
4. Test the new target

### Modifying Archives

To change archive format or contents:

1. Update `archives` section in `.goreleaser.yml`
2. Modify `files` list to include/exclude files
3. Change `format` or `format_overrides`

### Custom Release Notes

Modify the `changelog` section to:
- Change grouping rules
- Add/remove commit types
- Customize formatting

## Security

### Signing

GoReleaser supports artifact signing:
- GPG signing for binaries
- Cosign for container images
- SLSA provenance generation

### Checksums

All artifacts include SHA256 checksums for verification:
```bash
sha256sum -c ferrocp-1.0.0-checksums.txt
```

## Performance

### Build Optimization

The configuration includes:
- Link-time optimization (LTO)
- Target-specific CPU optimizations
- Debug symbol stripping
- Binary compression

### Parallel Builds

GoReleaser builds all targets in parallel, significantly reducing release time.

## Monitoring

### Release Metrics

Track release success with:
- GitHub release statistics
- Download counts
- Package manager metrics
- Docker pull statistics
