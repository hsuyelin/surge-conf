#!/bin/bash
# Surge Resources Sync Script
# Usage: ./sync.sh

set -e

# Colors for output
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Print status message (Cargo-style)
log_status() {
    printf "${CYAN}%12s${NC} %s\n" "$1" "$2"
}

log_success() {
    printf "${GREEN}%12s${NC} %s\n" "$1" "$2"
}

log_error() {
    printf "${RED}%12s${NC} %s\n" "$1" "$2"
}

# Check if Rust is installed
check_rust() {
    log_status "Checking" "Rust toolchain..."
    if ! command -v cargo &> /dev/null; then
        log_error "Error" "Rust is not installed. Please install it from https://rustup.rs/"
        exit 1
    fi
    log_success "Found" "$(cargo --version)"
}

# Build the sync tools
build_tools() {
    log_status "Building" "sync tools..."
    cd build
    cargo build --release --quiet
    cd ..
    log_success "Finished" "building sync tools"
}

# Run sync_icons
sync_icons() {
    log_status "Running" "sync_icons..."
    ./build/target/release/sync_icons
}

# Run sync_rules
sync_rules() {
    log_status "Running" "sync_rules..."
    ./build/target/release/sync_rules
}

# Run sync_modules
sync_modules() {
    log_status "Running" "sync_modules..."
    ./build/target/release/sync_modules
}

# Main function
main() {
    echo ""
    log_status "Syncing" "Surge resources from upstream..."
    echo ""

    check_rust
    build_tools
    sync_icons
    sync_rules
    sync_modules

    echo ""
    log_success "Done!" "All resources synced successfully"
    echo ""
}

main "$@"
