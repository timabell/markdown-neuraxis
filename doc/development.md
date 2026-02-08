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

### System Dependencies

The following system packages are required to build the application on Linux:

```bash
# Ubuntu/Debian
sudo apt install libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev

# Fedora/RHEL
sudo dnf install gtk3-devel glib2-devel cairo-devel pango-devel gdk-pixbuf2-devel atk-devel webkit2gtk4.1-devel libxdo-devel

# Arch Linux
sudo pacman -S gtk3 glib2 cairo pango gdk-pixbuf2 atk webkit2gtk libxdo
```

Or run the included setup script:
```bash
./dev-setup.sh
```

### Building the Application

```bash
cargo build
cargo test
cargo run <path-to-notes-folder>
```

## Testing

### Philosophy

The ultimate goal is **outside-in testing for near-perfect regression detection**. What matters is user-perceived functionality, not internal implementation details. As features accumulate, we need confidence that all previously delivered features remain intact. - See: https://0x5.uk/2024/03/27/why-do-automated-tests-matter/

The goal is comprehensive end-to-end coverage where tests verify complete features from the user's perspective. However full outside-in coverage is an accepted technical debt being addressed incrementally due to the exploratory nature of the architecture. Once it's clear that the architecture works as intended then there can be some effort to bring regression tests up to a better level.

### Current Approach

| Layer | Purpose | Location |
|-------|---------|----------|
| **Integration Tests** | Verify public API contracts and feature behavior | `crates/*/tests/` |
| **Inline Unit Tests** | Test implementation details where useful | `#[cfg(test)]` modules in source files |
| **Benchmarks** | Performance measurement (not regression enforcement) | `crates/markdown-neuraxis-engine/benches/` |

### Test Structure

- **Integration tests**: `crates/*/tests/*.rs` - separate files testing public API
- **Inline unit tests**: `#[cfg(test)] mod tests` at end of source files
- **Benchmarks**: `crates/markdown-neuraxis-engine/benches/` - Criterion performance tests
- **Test data**: `crates/*/test_data/` - real markdown files that triggered bugs

### Testing Libraries

| Library | Purpose |
|---------|---------|
| `rstest` | Parameterized/data-driven tests |
| `insta` | Snapshot testing for regression detection |
| `pretty_assertions` | Better diff output for assertion failures |
| `tempfile` | Temporary directory fixtures for I/O tests |
| `criterion` | Performance benchmarking |
| `dioxus-ssr` | Server-side rendering for component testing |

## Conventional Commits

This project uses [Conventional Commits](https://www.conventionalcommits.org/) for changelog generation via [git-cliff](https://git-cliff.org/).

**Not every commit needs a conventional prefix.** Only use prefixes for changes that should appear in release notes. A branch or PR only needs one conventional commit per user-facing change. Commits without prefixes ("unconventional commits") are filtered out of the changelog - use these freely for work-in-progress, refactoring steps, or internal changes that don't warrant a release note.

### Format

```
type(scope): description

[optional body]

[optional footer]
```

### Types

- `feat` - New features
- `fix` - Bug fixes
- `refactor` - Code refactoring
- `perf` - Performance improvements
- `doc` - Documentation changes
- `style` - Code style/formatting
- `chore` - Maintenance tasks
- `security` - Security fixes

### Scopes

Use scopes to indicate which component is affected:

- `android` - Android app (`android/`)
- `desktop` - Desktop app (Dioxus)
- `cli` - CLI tool (`crates/markdown-neuraxis-cli/`)
- `engine` - Core engine (`crates/markdown-neuraxis-engine/`)
- No scope - Core/cross-cutting changes

### Examples

```
feat(android): Add progressive file loading
fix(cli): Handle missing file gracefully
refactor(engine): Simplify markdown parser
feat: Add FileModel for workspace support
```

### Changelog Generation

Release notes are generated from conventional commits using git-cliff. Configuration is in [.github/cliff.toml](../.github/cliff.toml).

Commits are grouped by scope (Android, Desktop, Cli, Core) then by type (Features, Bug Fixes, etc.).

To generate a changelog:
```bash
git cliff --unreleased
```
