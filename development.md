# Development Setup

This document outlines the development environment setup for `markdown-neuraxis`.

## Prerequisites

### Rust Toolchain via asdf-vm

We use [asdf-vm](https://asdf-vm.com/) to manage the Rust toolchain instead of rustup. This provides better version management and consistency across development environments.

#### Install asdf-vm

If you don't have asdf installed:

```bash
# Follow installation instructions at https://asdf-vm.com/guide/getting-started.html
git clone https://github.com/asdf-vm/asdf.git ~/.asdf --branch v0.14.0
echo '. "$HOME/.asdf/asdf.sh"' >> ~/.bashrc
echo '. "$HOME/.asdf/completions/asdf.bash"' >> ~/.bashrc
# Restart your shell or source ~/.bashrc
```

#### Install Rust via asdf

```bash
# Add the Rust plugin
asdf plugin add rust https://github.com/code-lever/asdf-rust.git

# Install latest stable Rust
asdf install rust latest:1
asdf global rust latest:1

# Verify installation
rustc --version
cargo --version
```

### Additional Dependencies

Once we begin implementation, additional dependencies will be documented here based on the chosen GUI framework (Tauri + Svelte or pure Rust GUI).

## Project Structure

Currently the repository contains only documentation:
- `README.md` - Project overview
- `design.md` - Technical design document
- `development.md` - This file
- `LICENSE` - AGPL v3 license

Future structure will follow the planned architecture in `design.md`.

## Development Workflow

*To be documented once implementation begins*

## Testing

Following the outside-in testing approach outlined in `design.md`:
- Integration tests for all user-facing features
- Unit tests for internal modules and functions
- No feature delivery without passing tests

## Contributing

*To be documented once the project reaches a more mature state*