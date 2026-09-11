---
description: Regenerate base_addresses.rs from the game binaries via AOB scanning
---

Regenerate `lib/libdsr/src/codegen/base_addresses.rs`.

First check the prerequisites and report any that are missing instead of guessing:

- `.env` exists in the repo root and sets `DSR_INSTALLS_DIR`. If it is missing, tell the user to
  `cp .env.example .env` and edit it — do not create it with a guessed path.
- `DSR_INSTALLS_DIR` points at a directory that *contains* game install folders (a Steam library's
  `steamapps/common`), not at an install itself.

Then:

1. `git diff --stat lib/libdsr/src/codegen/base_addresses.rs` to confirm it is clean before you start,
   so the regeneration diff is readable.
2. Run `cargo xtask` and show the version banner it prints for each executable it scanned.
3. `git diff lib/libdsr/src/codegen/base_addresses.rs` and walk through what changed.

Interpreting the result:

- **A base address is missing from the output** — the AOB signature in
  `xtask/src/codegen/aob_scans.rs` did not match. That is the thing to fix, not the generated file.
- **A new `Version` variant appeared** — `libdsr::version::get_version()` still hardcodes
  `V1_03_1`, so the new version will not actually be selected at runtime. Point this out.
- **Addresses shifted for an existing version** — expected after a game patch; the signatures did
  their job.

Never edit `base_addresses.rs` by hand to make a diff look right.
