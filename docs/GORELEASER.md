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

1. **Before Hooks**: Install Rust targets and run tests
2. **Build**: Use custom Rust build scripts for cross-compilation
3. **Archive**: Create platform-specific archives
4. **Checksum**: Generate SHA256 checksums
5. **Release**: Create GitHub release with assets

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

1. **Create a tag**:
   ```bash
   git tag -a v1.0.0 -m "Release v1.0.0"
   git push origin v1.0.0
   ```

2. **Automatic Release**: The GitHub Action will automatically:
   - Build all platform binaries
   - Create archives and checksums
   - Generate release notes
   - Create a GitHub release
   - Upload all artifacts

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
- Triggers on version tags (`v*`)
- Sets up cross-compilation environment
- Runs GoReleaser
- Tests released binaries
- Supports dry-run mode

### Workflow Dispatch

You can manually trigger the workflow with options:
- **Dry-run**: Test without creating a release
- **Custom parameters**: Override default settings

## Package Managers

### Homebrew (macOS/Linux)

GoReleaser can automatically update a Homebrew tap:
```bash
brew tap loonghao/tap
brew install ferrocp
```

### Scoop (Windows)

Windows users can install via Scoop:
```bash
scoop bucket add ferrocp https://github.com/loonghao/scoop-bucket
scoop install ferrocp
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
