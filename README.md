<p align="center">
  <img src="./assets/mux-logo.svg" alt="mux logo" width="500" />
</p>

<h1 align="center">mux</h1>
<p align="center"><strong>Open-source project workflow CLI for C and C++ teams.</strong></p>

<p align="center">
  <a href="https://github.com/DHANUSH-web/mux/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/DHANUSH-web/mux/ci.yml?label=CI"></a>
  <a href="https://github.com/DHANUSH-web/mux/tags"><img alt="Latest Tag" src="https://img.shields.io/github/v/tag/DHANUSH-web/mux?sort=semver&label=release"></a>
  <a href="https://github.com/DHANUSH-web/mux/blob/main/LICENSE"><img alt="License: GPL-3.0-only" src="https://img.shields.io/badge/license-GPL--3.0--only-blue.svg"></a>
  <img alt="Language" src="https://img.shields.io/badge/language-Rust-orange">
  <img alt="CMake" src="https://img.shields.io/badge/build-CMake-064F8C">
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-0ea5e9">
</p>

`mux` gives C and C++ developers a Cargo-like command loop: scaffold, build, run, test, and clean with consistent CMake preset conventions.

## Why teams use mux

- Faster onboarding for native projects with a standard layout and commands.
- Clean build/test flow across debug and release without custom scripts.
- C template with Unity tests ready out of the box.
- C++ template with GoogleTest wired through `FetchContent`.
- Dependency vendoring workflow for C via `mux add` and `mux remove`.

## Quick Start

```bash
# Build mux
cargo build

# Create and enter a C project (default template)
mux init hello_c
cd hello_c

# Build, run, test
mux build
mux run
mux test
```

C++ project:

```bash
mux init hello_cpp --cpp
cd hello_cpp
mux build
mux run
mux test
```

## Install

From repository root:

```bash
cargo install --path .
mux --version
```

Prebuilt binaries:

- Download latest assets: `https://github.com/DHANUSH-web/mux/releases/latest`

Or run without global install:

```bash
cargo run -- --help
```

## CI/CD

- CI: `.github/workflows/ci.yml`
- Release: `.github/workflows/release.yml`
- CI runs on pushes to `main` and pull requests, and validates format/check/tests plus C/C++ smoke flows.
- Release builds Linux/macOS/Windows binaries and uploads them to GitHub Releases.
- Trigger a release by pushing a version tag, for example: `v0.1.0`.

## Command Surface

```text
mux init <project-name> [--lang c|cpp] [--cpp]
mux build [--release|--all]
mux run [--release]
mux test [--release|--all]
mux clean [--release|--all]
mux add <lib> [--target <cmake-target>]      # C template only
mux remove <lib-name>                         # C template only
```

Notes:

- `--all` and `--release` are mutually exclusive where both are supported.
- `build`, `run`, `test`, and `clean` must run inside a mux-generated project.

## Templates

### C template

```text
<project>/
  CMakeLists.txt
  CMakePresets.json
  src/main.c
  tests/test_main.c
  lib/unity/unity.h
  lib/unity/unity.c
```

- Targets: `main`, `test_main`
- Test framework: Unity
- Supports `mux add` and `mux remove`

### C++ template

```text
<project>/
  CMakeLists.txt
  CMakePresets.json
  src/main.cpp
  tests/test_main.cpp
  lib/README.md
```

- Targets: `main`, `test_main`
- Test framework: GoogleTest via CMake `FetchContent`
- `mux add` is intentionally not enabled yet

## C Dependency Management (C template only)

Add dependency:

```bash
mux add <lib>
```

Supported formats:

- Local path: `mux add ../my_c_lib`
- Git URL: `mux add https://github.com/user/repo.git`
- GitHub shorthand: `mux add owner/repo`
- Repository shorthand: `mux add repo` (expands to `https://github.com/repo/repo.git`)

Optional target override:

```bash
mux add owner/repo --target actual_cmake_target
```

Remove dependency:

```bash
mux remove <lib-name>
```

`mux` updates vendored files under `lib/`, `CMakeLists.txt`, and tracks metadata in `mux.lock`.

## Build and Output Conventions

- `CMakePresets.json` version: 6
- Presets: `unix-debug`, `unix-release`, `windows-debug`, `windows-release`
- Build outputs: `out/<platform>-<profile>`

Examples:

- `out/unix-debug`
- `out/unix-release`
- `out/windows-debug`
- `out/windows-release`

## Requirements

- Rust toolchain
- CMake 3.20+
- C/C++ compiler toolchain
- Internet access for first C++ configure/build (GoogleTest download)

## Contributing

Issues and pull requests are welcome.

Read these first:

- `CONTRIBUTING.md`
- `CODE_OF_CONDUCT.md`
- `SECURITY.md`

For local validation after behavior/template changes:

```bash
cargo fmt
cargo check
```

Then run smoke tests for:

- C flow: `init/build/run/test`
- C++ flow: `init --cpp`, `build`, `test`
- C add/remove flow: `add`, `remove`, `build`, `test`

## Getting Help

- Questions and usage help: open a GitHub issue with the `question` label.
- Bugs: use the bug report template in `.github/ISSUE_TEMPLATE/bug_report.yml`.
- Security issues: follow `SECURITY.md` and report privately.

## Maintainer

- Project owner: `@DHANUSH-web`
- Repository: `https://github.com/DHANUSH-web/mux`

## License

This project is licensed under **GPL-3.0-only**. See `LICENSE`.
