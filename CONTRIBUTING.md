# Contributing to mux

Thanks for contributing to `mux`.

## Before you start

- Open an issue first for large changes.
- Keep CLI semantics stable unless the change is explicitly discussed.
- If behavior changes, update docs in the same PR: `README.md`, `.codex/AGENT.md`, and clap help text in source.

## Development setup

```bash
git clone <your-fork-url>
cd mux
cargo check
```

## Coding guidelines

- Keep changes small and focused.
- Prefer clear, testable behavior over clever abstractions.
- Preserve template contracts for generated C/C++ projects.
- Do not break command flag contracts (`--all` and `--release` exclusivity).

## Validation checklist

Run before opening a PR:

```bash
cargo fmt
cargo check
```

Also run smoke tests for affected flows:

- C template: `init/build/run/test`
- C++ template: `init --cpp`, `build`, `test`
- C add/remove flows when touched

## Pull request process

- Use a clear title and explain user-visible behavior changes.
- Include reproduction steps for fixes.
- Include before/after examples for CLI output if relevant.
- Link related issues in PR description.

## Reporting bugs

Include:

- OS and compiler/toolchain details
- command used
- full output/error logs
- minimal reproduction steps
