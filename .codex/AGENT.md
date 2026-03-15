# AGENT.md - mux Project Context

Persistent context for AI agents working on `mux`.

## Project Summary
- Project: `mux`
- Language: Rust (binary crate)
- Purpose: CLI to scaffold and manage C/C++ projects with CMake presets
- Root: `$HOME/RustroverProjects/mux`

## CLI Scope
- `mux init <project-name>`
- `mux init <project-name> --lang c|cpp`
- `mux init <project-name> --cpp` (shortcut for C++)
- `mux build [--release|--all]`
- `mux run [--release]`
- `mux test [--release|--all]`
- `mux clean [--release|--all]`
- `mux add <lib> [--target <cmake-target>]` (C template only)
- `mux remove <lib-name>` (C template only)

## Template Contracts

### C template
- `src/main.c` (target: `main`)
- `tests/test_main.c` (target: `test_main`)
- `lib/unity/unity.h`
- `lib/unity/unity.c`
- test framework: Unity
- supports dependency vendoring with `mux add`
- supports dependency removal with `mux remove`
- writes dependency metadata into `mux.lock`

### C++ template
- `src/main.cpp` (target: `main`)
- `tests/test_main.cpp` (target: `test_main`)
- `lib/README.md`
- test framework: GoogleTest via CMake `FetchContent`
- `mux add` is intentionally not enabled yet

## Build/Test Conventions
- Presets use `<platform>-<profile>` naming
  - platform: `unix` or `windows`
  - profile: `debug` or `release`
- Build outputs under `out/<platform>-<profile>`
- `--all` and `--release` are mutually exclusive where both exist

## CMake Contract
- Uses `CMakePresets.json` (version 6)
- Presets:
  - `unix-debug`, `unix-release`
  - `windows-debug`, `windows-release`
- C++ template downloads GoogleTest on first configure/build

## Rust Implementation
- Source: `src/main.rs`
- Dependencies: `clap` (derive), `anyhow`
- Key functions:
  - `init_project`
  - `ensure_project_root`
  - `ensure_c_project_root`
  - `add_dependency`
  - `remove_dependency`
  - `configure_and_build`
  - `run_ctest`
  - `run_executable`
  - `clean_outputs`

## Required Validation After Template/CLI Changes
1. `cargo fmt`
2. `cargo check`
3. Smoke test C template (`init/build/run/test`)
4. Smoke test C++ template (`init --cpp`, `build`, `test`)
5. Smoke test C add flow (`init --lang c`, `add`, `build`, `test`)
6. Smoke test C remove flow (`remove`, `build`, `test`)

## CI/CD Contract
- CI workflow file: `.github/workflows/ci.yml`
- Release workflow file: `.github/workflows/release.yml`
- CI expectations:
  - `cargo fmt --all -- --check`
  - `cargo check --all-targets --locked`
  - `cargo test --all-targets --locked`
  - Ubuntu smoke coverage for C, C++, C add/remove flows
- Release expectations:
  - Triggered by version tags (`v*`)
  - Builds release binaries for:
    - `x86_64-unknown-linux-gnu`
    - `x86_64-apple-darwin`
    - `x86_64-pc-windows-msvc`
  - Publishes zipped/tarred artifacts plus SHA256 checksums to GitHub Releases

## Change Hygiene
- Keep command semantics stable unless explicitly requested.
- If behavior changes, update in same change:
  - `README.md`
  - `.codex/AGENT.md`
  - `--help` text (via clap definitions)
