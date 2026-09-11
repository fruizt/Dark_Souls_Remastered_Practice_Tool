---
description: Pre-release checklist — verify the repo and build artifacts are ship-ready
---

Run through the release checklist for this repo and report a pass/fail list. Do not fix anything
unless I ask — just tell me what is wrong.

**Repo hygiene**

- `git status` is clean, and `git ls-files | grep -i env` returns only `.env.example` files.
- `Cargo.lock` is tracked (this workspace ships executables).
- No `[patch.'crates-io']` in a member manifest — cargo ignores those; it belongs in the root.

**Build**

- `cargo build --release` succeeds.
- `cargo clippy --all-targets` — report warning count, call out anything in `libdsr`.
- `cargo test` passes.
- Both artifacts exist in `target/release/`: `dark_souls_remastered_tool.exe` and
  `dark_souls_remastered_tool_binaries.dll`.

**Config**

- `dark_souls_remastered_tool.toml` parses — every `flag` string in it has a matching
  `FlagSpec::try_from` arm, and every `indicator` name is one the render loop actually draws
  (`fps` and `animation` parse but draw nothing).
- No hotkey is bound twice in the shipped config.

**Docs**

- The README's Features table, "Available flags" list, and default-hotkey table all match what
  `config.rs` and the shipped `.toml` actually support. This drifts easily — check it properly rather
  than assuming.
- The roadmap's "not implemented" list does not claim something that now works.
- `LICENSE` is present and the README's license section matches it.

**Release archive** — remind me that a release needs the two build artifacts *plus*
`dark_souls_remastered_tool.toml`, since the overlay loads an empty widget list without it.
