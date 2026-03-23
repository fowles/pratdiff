# CLAUDE.md

## Project Overview

**pratdiff** is a colorful diff CLI tool in Rust implementing the Patience Diff algorithm (with a histogram extension for non-unique lines). It supports line-level and token-level colorization for files and directory trees.

## Architecture

```
src/
├── lib.rs          - Core Patience Diff algorithm
├── bin/pratdiff.rs - CLI entry point (clap-based)
├── diff.rs         - Core data structures: DiffItem, Hunk, Diffs
├── files.rs        - File/directory I/O and diff dispatch
├── printer.rs      - Output formatting and colorization
└── style.rs        - Color style definitions (owo_colors)
```

**Data flow:** `bin/pratdiff.rs` → `files.rs` → `lib.rs` (diff algorithm) → `printer.rs` (render)

## Build & Test

```bash
cargo build --release
cargo test                    # 12 unit tests in lib.rs + integration tests
cargo test <test_name>        # Run a specific test
```

## Key Design Decisions

- **Token-level diffs**: `tokenize_lines()` in `lib.rs` applies the same patience diff recursively at the token level for changed lines. Tokens include: whitespace, numbers, identifiers, and symbols.
- **Byte-oriented diff core**: `lib.rs` operates directly on `&[u8]` slices. Both line-level and token-level diffing use the same concrete `diff(&[&[u8]], &[&[u8]])` function.
- **Histogram fallback**: When no unique lines exist for patience diff, falls back to histogram diff (counts occurrences, prefers rarer lines).
- **Common prefix stripping**: `--verbose-paths` disables common path prefix removal in output headers.

## Notable Behaviors

- `-` as filename means stdin
- Binary files (non-UTF-8) are detected and reported but not diffed
- Unix inode checks prevent diffing a file against itself
- Color output auto-detects terminal; controllable via `--color=always|never|auto`
