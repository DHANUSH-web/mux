# AGENT.md - mux Project Context

Workspace path: `$HOME/RustroverProjects/mux`

Use this as high-level context for AI agents. Detailed canonical instructions are in:
- `./.codex/AGENT.md`

## Project

`mux` is a Rust CLI for scaffolding and managing C/C++ projects with CMake presets.

Supported init templates:
- C (`--lang c`, default) with Unity tests
- C++ (`--lang cpp` or `--cpp`) with GoogleTest

Core commands:
- `mux init <project-name> [--lang c|cpp|--cpp]`
- `mux build [--release|--all]`
- `mux run [--release]`
- `mux test [--release|--all]`
- `mux clean [--release|--all]`
- `mux add <lib> [--target <cmake-target>]` (C projects only)
- `mux remove <lib-name>` (C projects only)

## Contract Highlights

- Targets must stay:
  - `main`
  - `test_main`
- Build output stays under:
  - `out/<platform>-<profile>`
- Presets:
  - `unix-debug`, `unix-release`
  - `windows-debug`, `windows-release`

## Change Rules

When CLI behavior or templates change:
1. update `README.md`
2. update `./.codex/AGENT.md`
3. run `cargo fmt && cargo check`
4. smoke test affected template flows
