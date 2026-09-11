---
description: Wire up a new game memory bitflag end to end, from pointer chain to README
---

Add a new toggleable bitflag to the practice tool: **$ARGUMENTS**

Work through all four layers. Do not stop after the first one that compiles.

1. **`lib/libdsr/src/pointers.rs`** — add a `Bitflag<u8>` field to `PointerChains` and construct it
   in the `From<BaseAddresses>` impl with the `bitflag!` macro. If you do not have a verified
   offset/mask for this flag, say so and stop here rather than inventing one — a wrong write can
   crash the game or corrupt a save.
2. **`tool/src/config.rs`** — add an arm to `FlagSpec::try_from`, mapping the config string to a
   human-readable label and a getter closure. Check whether the flag is already present as a
   commented-out arm; if so, uncomment rather than duplicating.
3. **`dark_souls_remastered_tool.toml`** — add the command with a default hotkey, if it deserves one.
   Check the existing hotkeys first so you do not collide.
4. **Docs** — add it to the Features table and the "Available flags" list in `README.md`, and remove
   it from the roadmap's not-implemented list if it was there.

Then run `cargo check`. Report that it compiles — and state explicitly that the flag is unverified
until the user injects into a running game and toggles it, since `cargo check` cannot validate an
offset.
