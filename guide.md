# mmdflux Development Guide

A comprehensive guide to setup, install, test, and build the mmdflux project.

## Project Overview

**mmdflux** is a Rust CLI tool and library that parses Mermaid diagrams and renders them as:
- **Terminal text** (Unicode/ASCII)
- **SVG** (Scalable Vector Graphics)
- **MMDS JSON** (Machine-Mediated Diagram Specification)

Supported diagram types: flowchart, class, and sequence diagrams.

Key features:
- No runtime dependencies (single binary)
- Native orthogonal routing
- Structured JSON output for tooling and agents
- Typed diagram edits and events

---

## Prerequisites

Ensure you have the following installed:

### Required Tools
- **Rust** (stable and nightly toolchains)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup toolchain install stable nightly
  ```
- **Git**
- **Just** (task runner)
  ```bash
  cargo install just
  ```

### Optional Tools (for development)
- **cargo-nextest** (parallel test runner) — automatically used via `just test`
- **cargo-edit** (for easy dependency management)

### Verify Installation
```bash
rustc --version
cargo --version
just --version
```

---

## Setup

### 1. Clone the Repository
```bash
git clone https://github.com/kevinswiber/mmdflux.git
cd mmdflux
```

### 2. Verify Prerequisites
```bash
# All tools should print their versions
rustc --version
cargo --version
git --version
just --version
```

### 3. Optional: Setup Debug Dependencies
To compare mmdflux layout against dagre.js v0.8.5:
```bash
./scripts/setup-debug-deps.sh  # Clones dagre and mermaid to deps/
```

---

## Installation & Building

### Debug Build
Compiles with debugging symbols and without optimizations:
```bash
just build
# or
cargo build
```

Output binary: `target/debug/mmdflux`

### Release Build
Optimized build for production use:
```bash
just release
# or
cargo build --release
```

Output binary: `target/release/mmdflux`

### Install Globally
Install the compiled binary to your system:
```bash
cargo install --path .
# Available as `mmdflux` in any terminal
```

---

## Running Tests

### Run All Tests
```bash
just test
# Runs with nextest (parallel execution)
```

### Run a Specific Test File
```bash
just test-file integration
# Runs all tests in tests/integration.rs
```

### Run Tests Matching a Pattern
```bash
just test -E 'test(test_name)'
# Example: just test -E 'test(flowchart)'
```

### Run Tests in CI Mode
```bash
just test-ci
# Disables fail-fast, verbose output
```

### Text Snapshot Regeneration
Regenerate text snapshots for each diagram type:
```bash
# Flowchart
GENERATE_TEXT_SNAPSHOTS=1 cargo nextest run -E 'test(generate_baseline_snapshots)'

# Class diagrams
GENERATE_CLASS_TEXT_SNAPSHOTS=1 cargo nextest run --test compliance_class -E 'test(class_text_snapshots)'

# Sequence diagrams
GENERATE_SEQUENCE_TEXT_SNAPSHOTS=1 cargo nextest run --test compliance_sequence -E 'test(sequence_text_snapshots)'
```

### SVG Snapshot Regeneration
```bash
# Flowchart
GENERATE_SVG_SNAPSHOTS=1 cargo nextest run -E 'test(svg_snapshot_all_fixtures)'

# Class diagrams
GENERATE_CLASS_SVG_SNAPSHOTS=1 cargo nextest run --test compliance_class -E 'test(class_svg_snapshots)'

# Sequence diagrams
GENERATE_SEQUENCE_TEXT_SNAPSHOTS=1 cargo nextest run --test compliance_sequence -E 'test(sequence_text_snapshots)'
```

### Dagre Parity Tests
Compare layout against dagre.js:
```bash
cargo nextest run -E 'test(dagre_parity)'
```

---

## Code Quality & Verification

### Lint & Format Checks
```bash
just lint
# Runs: clippy, architecture boundaries check, format verification
```

### Format Code
```bash
just fmt
# Auto-formats all code with rustfmt
```

### Check Format (No Changes)
```bash
just fmt-check
# Verifies formatting without modifying files
```

### Auto-Fix Issues
```bash
just fix
# Runs clippy with auto-fix and formats code
```

### Full Project Verification
Run the complete verification suite (lint, tests, architecture):
```bash
just check
# Equivalent to: lint + test + architecture check
```

---

## Development Workflow

### Common Development Commands

```bash
# View available recipes
just --list

# Run the CLI on a diagram file
just run diagram.mmd

# Run CLI with debug output
cargo run -- --debug diagram.mmd

# Read from stdin
echo 'graph LR\nA-->B' | cargo run

# Run with ASCII output
cargo run -- --ascii diagram.mmd

# Compile and watch for changes (via cargo-watch)
cargo watch -x "build"

# Run a specific test in watch mode
cargo watch -x "test test_name"
```

### Architecture Verification

The project uses semantic architecture boundaries defined in `boundaries.toml`:

```bash
just architecture-check
# Verifies module dependency rules
```

View the dependency graph:
```bash
just architecture-graph
# Prints dependency graph as Mermaid

just architecture-explain --edge <source> <target>
# Inspect a specific edge

just architecture-explain --boundary <name>
# Inspect a boundary
```

Host an interactive architecture linter:
```bash
just architecture-host
# Access at http://localhost:3000
```

---

## Debug Infrastructure

### Enable Tracing

The project uses `tracing` for diagnostics. Enable with environment variables:

```bash
# CLI tracing
mmdflux --log "<FILTER>" diagram.mmd
# Examples:
mmdflux --log "mmdflux::runtime=debug" diagram.mmd
mmdflux --log "mmdflux::engines::graph::algorithms=trace" diagram.mmd

# Alternative via env var
MMDFLUX_LOG=mmdflux::runtime=debug mmdflux diagram.mmd

# Select format (compact, pretty, json)
mmdflux --log "info" --log-format pretty diagram.mmd

# Write to file
mmdflux --log "debug" --log-file trace.log diagram.mmd

# For xtask commands
MMDFLUX_XTASK_LOG=debug just check
```

### Debug Dumpers

Deterministic files and outputs via environment variables:

```bash
# Layout JSON
MMDFLUX_DEBUG_LAYOUT=1 mmdflux diagram.mmd > layout.json
MMDFLUX_DEBUG_LAYOUT=layout.json mmdflux diagram.mmd  # Write to file

# Pipeline stages (JSONL)
MMDFLUX_DEBUG_PIPELINE=1 mmdflux diagram.mmd > pipeline.jsonl
MMDFLUX_DEBUG_PIPELINE=pipeline.jsonl mmdflux diagram.mmd  # File mode appends

# Border node parity
MMDFLUX_DEBUG_BORDER_NODES=1 mmdflux diagram.mmd 2>&1

# SVG auto-theme probe
MMDFLUX_DEBUG_SVG_THEME_AUTO=theme-probe.txt mmdflux diagram.mmd
```

### Debug Scripts

The `scripts/` directory includes utilities for debugging:

```bash
node scripts/dump-dagre-layout.js      # Run dagre.js layout
node scripts/dump-dagre-pipeline.js    # Trace dagre pipeline stages
node scripts/dump-dagre-borders.js     # Extract dagre border nodes
node scripts/dump-dagre-order.js       # Dump node order per rank
```

---

## Project Structure

```
mmdflux/
├── src/                         # Source code
│   ├── main.rs                 # CLI entry point
│   ├── lib.rs                  # Library facade
│   ├── builtins.rs             # Built-in shapes & registry
│   ├── diagrams/               # Diagram type implementations
│   ├── engines/                # Layout engines (graph solver)
│   ├── mermaid/                # Mermaid parser
│   ├── render/                 # Output formatters (text/SVG/JSON)
│   ├── runtime/                # Orchestration layer
│   └── ...
├── tests/                       # Integration & unit tests
│   ├── fixtures/               # Test diagram files
│   ├── snapshots/              # Text output snapshots
│   ├── svg-snapshots/          # SVG output snapshots
│   └── ...
├── Cargo.toml                   # Rust manifest
├── Justfile                     # Task definitions
├── AGENTS.md                    # AI assistant guidelines
├── CLAUDE.md                    # Claude-specific instructions
├── cog.toml                     # Conventional commits config
└── ...
```

## Test Fixtures

Test fixtures are organized by diagram type:
- `tests/fixtures/flowchart/*.mmd` — Flowchart fixtures
- `tests/fixtures/class/*.mmd` — Class diagram fixtures
- `tests/fixtures/sequence/*.mmd` — Sequence diagram fixtures

Snapshots follow the same structure:
- `tests/snapshots/flowchart/*.txt` — Text output snapshots
- `tests/svg-snapshots/flowchart/*.svg` — SVG output snapshots

---

## Commit Conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/), enforced by cocogitto:

**Format:**
```
<type>(<optional scope>): <subject>
```

**Types:** `feat`, `fix`, `perf`, `revert`, `docs`, `test`, `build`, `ci`, `refactor`, `chore`, `style`

**Scopes:** `wasm`, `xtask`, `web`, `mmds-core`, `mmds-excalidraw`, `mmds-tldraw` (for monorepo packages)

**Rules:**
- Header: ≤ 100 characters
- Subject: lowercase, no period, imperative mood
- Include a body for non-trivial changes (explain **what** and **why**)
- Use `git commit` (the commit-msg hook validates automatically)

**Example:**
```
feat(mermaid): add flowchart subgraph support

Implement parsing and rendering of Mermaid flowchart subgraphs
with nested direction overrides. Extends the AST with SubgraphNode
and updates the layout engine to compute subgraph bounds.

Closes #42
```

---

## Branch Conventions

This project uses [Conventional Branch](https://conventional-branch.github.io/) format, enforced by a pre-push hook:

**Format:**
```
<type>/<description>
```

**Types:** `feat/`, `fix/`, `hotfix/`, `release/`, `chore/`

**Rules:**
- Use lowercase letters, numbers, and hyphens only
- Include issue numbers when applicable: `fix/issue-42-fix-layout-bug`
- Keep descriptions concise: `fix/lr-routing-regression`

---

## Common Issues & Troubleshooting

### Build Failures

**Issue:** `rustc` version mismatch
```bash
# Update toolchains
rustup update stable nightly
rustupcontinued default stable
```

**Issue:** Out-of-date Cargo.lock
```bash
rm Cargo.lock
cargo build
```

### Test Failures

**Issue:** Snapshot mismatches after code changes
```bash
# Regenerate snapshots (use with caution)
GENERATE_TEXT_SNAPSHOTS=1 cargo nextest run -E 'test(generate_baseline_snapshots)'
GENERATE_SVG_SNAPSHOTS=1 cargo nextest run -E 'test(svg_snapshot_all_fixtures)'

# Then review and commit changes
git diff tests/snapshots/
git add tests/snapshots/
```

**Issue:** Tests timeout
```bash
# Increase timeout (in Cargo.toml or via flag)
cargo nextest run -- --test-threads=1

# Or run a specific test
just test -E 'test(specific_test)'
```

### Development Environment

**Issue:** Just command not found
```bash
cargo install just
# or on macOS
brew install just
```

**Issue:** Nightly formatting issues
```bash
# Update nightly
rustup update nightly
# Regenerate lock files if needed
cargo +nightly update
```

---

## Documentation

Key documentation files:
- [README.md](README.md) — Project overview and quick start
- [AGENTS.md](AGENTS.md) — Detailed AI assistant guidelines
- [docs/mmds.md](docs/mmds.md) — MMDS JSON specification
- [docs/architecture/dependency-rules.md](docs/architecture/dependency-rules.md) — Module ownership rules
- [docs/development/mermaid-parity.md](docs/development/mermaid-parity.md) — Mermaid parity testing

---

## Next Steps

1. **Setup:** Follow the [Prerequisites](#prerequisites) and [Setup](#setup) sections
2. **Build:** Run `just build` to compile the project
3. **Test:** Run `just test` to verify everything works
4. **Explore:** Check [Common Development Commands](#common-development-commands)
5. **Read:** Review [AGENTS.md](AGENTS.md) and [docs/architecture/dependency-rules.md](docs/architecture/dependency-rules.md)

---

## Contributing

When making changes:
1. Create a conventional branch: `feat/feature-name` or `fix/issue-number-description`
2. Make changes and run `just check` to verify
3. Commit with conventional commit format
4. Open a pull request referencing the issue

---

## Resources

- **GitHub:** https://github.com/kevinswiber/mmdflux
- **Crates.io:** https://crates.io/crates/mmdflux
- **Docs.rs:** https://docs.rs/mmdflux
- **Playground:** https://play.mmdflux.com
- **Gallery:** [docs/gallery.md](docs/gallery.md)

---

*Last updated: May 2026*
