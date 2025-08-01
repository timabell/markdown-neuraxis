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
    libgdk-pixbuf2.0-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libatk1.0-dev \
    libsoup2.4-dev \
    libjavascriptcoregtk-4.0-dev \
    libwebkit2gtk-4.0-dev

echo "Development dependencies installed successfully!"
echo "You can now run 'cargo build' to build the application."
