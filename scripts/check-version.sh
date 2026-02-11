#!/bin/bash

# This script checks if the provided version matches the versions in Cargo.toml and META.json
# Usage: source ./scripts/check-version.sh <version> [--warn-only]
# After sourcing, the following variables will be available:
#   - CARGO_VERSION: Version from Cargo.toml
#   - META_VERSION: Version from META.json
#   - HAS_MISMATCH: true if versions don't match, false otherwise

VERSION_TO_CHECK=$1
WARN_ONLY=false

# Check for --warn-only flag
if [ "$2" = "--warn-only" ]; then
    WARN_ONLY=true
fi

if [ -z "$VERSION_TO_CHECK" ]; then
    echo "Error: Version argument required for check-version.sh"
    exit 1
fi

# Extract versions from Cargo.toml and META.json
CARGO_VERSION=$(grep -E '^version = ' Cargo.toml | head -n 1 | sed -E 's/version = "(.*)"/\1/')
META_VERSION=$(jq -r '.version' META.json)

# Check for version mismatches
HAS_MISMATCH=false

if [ "$VERSION_TO_CHECK" != "$CARGO_VERSION" ]; then
    echo ""
    if [ "$WARN_ONLY" = true ]; then
        echo "⚠️  Warning: Cargo.toml has version $CARGO_VERSION"
    else
        echo "❌ Error: Cargo.toml has version $CARGO_VERSION"
    fi
    HAS_MISMATCH=true
fi

if [ "$VERSION_TO_CHECK" != "$META_VERSION" ]; then
    echo ""
    if [ "$WARN_ONLY" = true ]; then
        echo "⚠️  Warning: META.json has version $META_VERSION"
    else
        echo "❌ Error: META.json has version $META_VERSION"
    fi
    HAS_MISMATCH=true
fi
