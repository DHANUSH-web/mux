# mux

`mux` is a Rust CLI to scaffold and manage C and C++ projects using CMake presets.

It provides a cargo/bun-like workflow for native projects:
- initialize project templates
- build debug/release presets
- run binaries
- run tests
- clean build outputs

## Features

- C template with Unity test scaffold
- C++ template with GoogleTest (gtest) scaffold
- C dependency vendoring with `mux add <lib>` into `lib/` (C template only)
- C dependency removal with `mux remove <lib>` and lock tracking in `mux.lock`
- Cross-platform preset naming and output layout
- Unified commands for build/run/test/clean

## Install / Build

From the `mux` project root:

```bash
cargo build
```

Optional install:

```bash
cargo install --path .
```

## CLI Commands

```bash
mux --help
```

### Initialize

Create C project (default):

```bash
mux init my_c_app
# or explicit
mux init my_c_app --lang c
```

Create C++ project with GoogleTest:

```bash
mux init my_cpp_app --lang cpp
# shortcut
mux init my_cpp_app --cpp
```

### Build

```bash
mux build
mux build --release
mux build --all
```

### Run

```bash
mux run
mux run --release
```

### Test

```bash
mux test
mux test --release
mux test --all
```

### Clean

```bash
mux clean
mux clean --release
mux clean --all
```

### Add Dependency (C Template Only)

```bash
mux add <lib>
```

Supported `<lib>` formats:
- local path: `mux add ../my_c_lib`
- git url: `mux add https://github.com/user/repo.git`
- GitHub shorthand:
  - `mux add owner/repo`
  - `mux add repo` (expanded to `https://github.com/repo/repo.git`)

Optional target override:

```bash
mux add owner/repo --target actual_cmake_target
```

`mux add` behavior:
- copies/clones dependency into `lib/<name>`
- updates root `CMakeLists.txt` with a dependency block
- links dependency to `main` and `test_main` when applicable
- currently supported only for C projects (`src/main.c`)

### Remove Dependency (C Template Only)

```bash
mux remove <lib-name>
```

`mux remove` behavior:
- removes `lib/<lib-name>` when present
- removes the corresponding mux-generated block from `CMakeLists.txt`
- removes the entry from `mux.lock`

## Generated Templates

### C Template

```text
<project>/
  CMakeLists.txt
  CMakePresets.json
  src/main.c
  tests/test_main.c
  lib/unity/unity.h
  lib/unity/unity.c
```

Targets:
- `main`
- `test_main`

Test framework:
- Unity

### C++ Template

```text
<project>/
  CMakeLists.txt
  CMakePresets.json
  src/main.cpp
  tests/test_main.cpp
  lib/README.md
```

Targets:
- `main`
- `test_main`

Test framework:
- GoogleTest via CMake `FetchContent`

## Build Output Layout

Build artifacts are stored in:

```text
out/<platform>-<profile>
```

Examples:
- `out/unix-debug`
- `out/unix-release`
- `out/windows-debug`
- `out/windows-release`

## Prerequisites

- Rust toolchain
- CMake 3.20+
- C/C++ compiler toolchain
- Internet access for first C++ configure/build (to download GoogleTest)

## Typical Workflow

```bash
# C example
mux init demo_c
cd demo_c
mux build
mux run
mux test

# C++ example
cd ..
mux init demo_cpp --cpp
cd demo_cpp
mux build
mux run
mux test
```

## Notes

- `build/test/run/clean` must be executed inside a mux-generated project.
- `--all` and `--release` are mutually exclusive on commands that accept both.
- `mux add`/`mux remove` currently support only C templates.
- `mux.lock` is a tab-separated lock file maintained by mux for vendored deps.
- Additional agent context files:
  - `AGENTS.md`
  - `.codex/AGENT.md`
