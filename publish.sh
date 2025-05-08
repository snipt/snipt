#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}==>${NC} $1"
}

print_error() {
    echo -e "${RED}Error:${NC} $1"
}

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    print_error "cargo is not installed"
    exit 1
fi

# Parse command line arguments
DRY_RUN=""
if [ "$1" == "--dry-run" ]; then
    DRY_RUN="--dry-run"
    print_status "Running in dry-run mode"
fi

# Array of crates to publish in order
CRATES=("snipt-core" "snipt-daemon" "snipt-server" "snipt-ui" "snipt-cli")

# Function to publish a crate
publish_crate() {
    local crate=$1
    print_status "Publishing $crate..."
    
    if ! cargo publish -p "$crate" $DRY_RUN; then
        print_error "Failed to publish $crate"
        exit 1
    fi
    
    print_status "Successfully published $crate"
}

# Main publishing loop
for crate in "${CRATES[@]}"; do
    publish_crate "$crate"
done

print_status "All crates have been processed successfully!" 