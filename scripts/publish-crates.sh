#!/bin/bash
# Publish Rust crates to crates.io in dependency order

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Configuration
DRY_RUN=false
FORCE_PUBLISH=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --force)
            FORCE_PUBLISH=true
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check if CARGO_REGISTRY_TOKEN is set
if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
    log_error "CARGO_REGISTRY_TOKEN environment variable is not set"
    exit 1
fi

log_info "Publishing FerroCP crates to crates.io..."
if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Running in DRY-RUN mode"
fi

# Define crates in dependency order (dependencies first)
# This ensures that dependent crates are published after their dependencies
CRATES=(
    "crates/ferrocp-types"
    "crates/ferrocp-zerocopy"
    "crates/ferrocp-io"
    "crates/ferrocp-device"
    "crates/ferrocp-compression"
    "crates/ferrocp-network"
    "crates/ferrocp-sync"
    "crates/ferrocp-config"
    "crates/ferrocp-engine"
    "crates/ferrocp-cli"
    # Note: ferrocp-python and ferrocp-tests are not published to crates.io
)

# Function to check if a crate should be published
should_publish_crate() {
    local crate_path=$1
    local cargo_toml="$crate_path/Cargo.toml"
    
    # Check if Cargo.toml exists
    if [[ ! -f "$cargo_toml" ]]; then
        log_warning "Cargo.toml not found at $cargo_toml"
        return 1
    fi
    
    # Check if crate has publish = false
    if grep -q "publish = false" "$cargo_toml"; then
        log_info "Skipping $crate_path (publish = false)"
        return 1
    fi
    
    # Check if it's a binary-only crate (CLI tools typically shouldn't be published as libraries)
    if [[ "$crate_path" == *"-cli" ]]; then
        log_info "Skipping $crate_path (CLI binary crate)"
        return 1
    fi
    
    # Check if it's a test crate
    if [[ "$crate_path" == *"-tests" ]]; then
        log_info "Skipping $crate_path (test crate)"
        return 1
    fi
    
    # Check if it's a Python binding crate
    if [[ "$crate_path" == *"-python" ]]; then
        log_info "Skipping $crate_path (Python binding crate)"
        return 1
    fi
    
    return 0
}

# Function to get crate name from Cargo.toml
get_crate_name() {
    local crate_path=$1
    local cargo_toml="$crate_path/Cargo.toml"
    
    # Extract name from Cargo.toml
    grep "^name = " "$cargo_toml" | head -1 | sed 's/name = "\(.*\)"/\1/'
}

# Function to get crate version from Cargo.toml
get_crate_version() {
    local crate_path=$1
    local cargo_toml="$crate_path/Cargo.toml"
    
    # Check if version is workspace-inherited
    if grep -q "version.workspace = true" "$cargo_toml"; then
        # Get version from workspace Cargo.toml
        grep "^version = " Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'
    else
        # Get version from local Cargo.toml
        grep "^version = " "$cargo_toml" | head -1 | sed 's/version = "\(.*\)"/\1/'
    fi
}

# Function to check if a crate version already exists on crates.io
crate_version_exists() {
    local crate_name=$1
    local version=$2
    
    # Query crates.io API to check if version exists
    local response=$(curl -s "https://crates.io/api/v1/crates/$crate_name" || echo "")
    
    if [[ -z "$response" ]]; then
        # If we can't query the API, assume it doesn't exist
        return 1
    fi
    
    # Check if the version exists in the response
    if echo "$response" | grep -q "\"num\":\"$version\""; then
        return 0
    else
        return 1
    fi
}

# Function to publish a single crate
publish_crate() {
    local crate_path=$1
    local crate_name=$(get_crate_name "$crate_path")
    local version=$(get_crate_version "$crate_path")
    
    log_info "Processing crate: $crate_name v$version"
    
    # Check if this version already exists on crates.io
    if crate_version_exists "$crate_name" "$version" && [[ "$FORCE_PUBLISH" != "true" ]]; then
        log_warning "Version $version of $crate_name already exists on crates.io, skipping"
        return 0
    fi
    
    # Change to crate directory
    pushd "$crate_path" > /dev/null
    
    # Prepare publish command
    local publish_cmd="cargo publish --no-verify"
    if [[ "$DRY_RUN" == "true" ]]; then
        publish_cmd="$publish_cmd --dry-run"
    fi
    
    log_info "Publishing $crate_name v$version..."
    log_info "Command: $publish_cmd"
    
    # Run the publish command
    if $publish_cmd; then
        if [[ "$DRY_RUN" == "true" ]]; then
            log_success "Dry-run successful for $crate_name v$version"
        else
            log_success "Published $crate_name v$version to crates.io"
            # Wait a bit to allow crates.io to process the upload
            sleep 10
        fi
    else
        log_error "Failed to publish $crate_name v$version"
        popd > /dev/null
        return 1
    fi
    
    popd > /dev/null
    return 0
}

# Main execution
main() {
    local failed_crates=()
    local published_crates=()
    local skipped_crates=()
    
    log_info "Starting crate publishing process..."
    
    # Publish each crate in dependency order
    for crate_path in "${CRATES[@]}"; do
        if should_publish_crate "$crate_path"; then
            if publish_crate "$crate_path"; then
                published_crates+=("$crate_path")
            else
                failed_crates+=("$crate_path")
            fi
        else
            skipped_crates+=("$crate_path")
        fi
        
        echo # Add spacing between crates
    done
    
    # Summary
    echo "=================================="
    log_info "Publishing Summary"
    echo "=================================="
    
    if [[ ${#published_crates[@]} -gt 0 ]]; then
        if [[ "$DRY_RUN" == "true" ]]; then
            log_success "Dry-run successful for ${#published_crates[@]} crates:"
        else
            log_success "Successfully published ${#published_crates[@]} crates:"
        fi
        for crate in "${published_crates[@]}"; do
            echo "  ✓ $crate"
        done
    fi
    
    if [[ ${#skipped_crates[@]} -gt 0 ]]; then
        log_info "Skipped ${#skipped_crates[@]} crates:"
        for crate in "${skipped_crates[@]}"; do
            echo "  - $crate"
        done
    fi
    
    if [[ ${#failed_crates[@]} -gt 0 ]]; then
        log_error "Failed to publish ${#failed_crates[@]} crates:"
        for crate in "${failed_crates[@]}"; do
            echo "  ✗ $crate"
        done
        echo
        log_error "Some crates failed to publish. Check the output above for details."
        exit 1
    fi
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_success "All crates passed dry-run validation!"
    else
        log_success "All eligible crates published successfully!"
    fi
}

# Run main function
main "$@"
