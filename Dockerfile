# Multi-stage Dockerfile for FerroCP
# This builds a minimal container with just the FerroCP binary

# Build stage - use the prebuilt binary from GoReleaser
FROM scratch AS runtime

# Copy the binary from the build context (provided by GoReleaser)
COPY ferrocp /usr/local/bin/ferrocp

# Set the binary as executable
USER 1000:1000

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/ferrocp"]

# Default command
CMD ["--help"]

# Metadata
LABEL org.opencontainers.image.title="FerroCP"
LABEL org.opencontainers.image.description="High-performance file copying tool built with Rust"
LABEL org.opencontainers.image.url="https://github.com/loonghao/FerroCP"
LABEL org.opencontainers.image.source="https://github.com/loonghao/FerroCP"
LABEL org.opencontainers.image.licenses="Apache-2.0"
