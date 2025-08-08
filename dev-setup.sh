#!/bin/bash -v

# Development setup script for markdown-neuraxis
# Installs system dependencies required for Dioxus desktop development on Linux

set -e # exit on error

echo "Setting up development environment for markdown-neuraxis..."

# Update package list
echo "Updating package list..."
sudo apt update

# Install GTK and related development libraries required by Dioxus desktop
echo "Installing GTK development libraries..."
sudo apt install -y \
    libgtk-3-dev \
    libglib2.0-dev \
    libwebkit2gtk-4.1-dev \
    libxdo-dev

echo "Installing Rust development tools..."
cargo install cargo-insta

echo "Development dependencies installed successfully!"
echo "You can now run 'cargo build' to build the application."
