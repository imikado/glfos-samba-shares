#!/usr/bin/env bash
# Development wrapper script for samba-share
# This script ensures the correct environment is set up before running with sudo

set -e

# Build if needed
if [ ! -f "target/debug/samba-share" ]; then
    echo "Building application..."
    cargo build
fi

# Check if we're in a nix develop shell
if [ -z "$XDG_DATA_DIRS" ] || [[ ! "$XDG_DATA_DIRS" =~ "gsettings-desktop-schemas" ]]; then
    echo "Warning: XDG_DATA_DIRS is not set correctly."
    echo "Please run this script from within 'nix develop'."
    echo ""
    echo "Attempting to set XDG_DATA_DIRS from nix..."

    # Try to get the paths from nix
    GSETTINGS_PATH=$(nix eval --raw nixpkgs#gsettings-desktop-schemas.outPath 2>/dev/null)/share/gsettings-schemas/gsettings-desktop-schemas-*
    GTK4_PATH=$(nix eval --raw nixpkgs#gtk4.outPath 2>/dev/null)/share/gsettings-schemas/gtk4-*

    export XDG_DATA_DIRS="${GSETTINGS_PATH}:${GTK4_PATH}:${XDG_DATA_DIRS}"
fi

echo "Running with sudo while preserving XDG_DATA_DIRS..."
echo "XDG_DATA_DIRS=$XDG_DATA_DIRS"
echo ""

# Run with sudo, preserving the XDG_DATA_DIRS environment variable
sudo env XDG_DATA_DIRS="$XDG_DATA_DIRS" ./target/debug/samba-share "$@"
