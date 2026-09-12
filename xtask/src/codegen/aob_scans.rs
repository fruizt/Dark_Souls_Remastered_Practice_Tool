use std::env;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

// use practice_tool_tasks::codegen::{self, aob_direct, aob_indirect, aob_indirect_twice};
use super::codegen::{self, aob_direct, aob_indirect_twice};

/// Every `DarkSoulsRemastered.exe` found one level below `DSR_INSTALLS_DIR`.
///
/// The variable points at a directory that *contains* game installs (a Steam
/// library's `steamapps/common`, say), not at an install itself, so that
/// several game versions sitting side by side can be scanned in one run.
fn patches_paths() -> Result<impl Iterator<Item = PathBuf>> {
    let base_path = env::var("DSR_INSTALLS_DIR").map(PathBuf::from).context(
        "DSR_INSTALLS_DIR is not set. Copy .env.example to .env and point it at the directory \
         containing your Dark Souls Remastered install.",
    )?;

    if !base_path.is_dir() {
        bail!("DSR_INSTALLS_DIR does not point at a directory: {}", base_path.display());
    }

    Ok(base_path
        .read_dir()
        .with_context(|| format!("Couldn't scan patches directory {}", base_path.display()))?
        .filter_map(Result::ok)
        .map(|dir| dir.path().join("DarkSoulsRemastered.exe")))
}

fn base_addresses_rs_path() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
        .join("lib")
        .join("libdsr")
        .join("src")
        .join("codegen")
        .join("base_addresses.rs")
}

pub fn get_base_addresses() -> Result<()> {
    // Names are FromSoftware's own, recovered from the RTTI type descriptors the
    // game ships (`.?AVGameDataMan@NS_FRPG@@` and friends). Keeping them means
    // the offset tables published by the community apply to these bases without
    // translation.
    let aobs = &[
        // Singletons.
        aob_indirect_twice(
            // Player game data: stats, souls, IGT. Was called `WorldChrMan` here, which
            // is a different object entirely — see `WorldChrMan` below.
            "GameDataMan",
            &["48 8B 05 xx xx xx xx 45 33 ED 48 8B F1 48 85 C0"],
            3,
            7,
            true,
        ),
        aob_indirect_twice(
            // The character manager. `+0x68` is ChrData1, which owns the per-character
            // flags, position and animation. Was called `CharacterFlags` here.
            "WorldChrMan",
            &["48 8B 05 xx xx xx xx 48 39 48 68 0F 94 C0 C3"],
            3,
            7,
            true,
        ),
        aob_indirect_twice(
            // Menu manager. `+0x24C` is the menu kick used for quitout. Was `BaseMenu`.
            "MenuMan",
            &["48 8B 05 xx xx xx xx 48 63 C9 89 54 88 30"],
            3,
            7,
            true,
        ),
        aob_indirect_twice(
            "ChrClassWarp",
            &["48 8B 05 ? ? ? ? 66 0F 7F 80 ? ? ? ? 0F 28 02 66 0F 7F 80 ? ? ? ? C6 80"],
            3,
            7,
            true,
        ),
        aob_indirect_twice(
            "EventFlagMan",
            &["48 8B 0D ? ? ? ? 99 33 C2 45 33 C0 2B C2 8D 50 F6"],
            3,
            7,
            true,
        ),
        aob_indirect_twice("CamMan", &["48 8B 05 ? ? ? ? 48 63 D1 48 8B 44 D0 08 C3"], 3, 7, true),
        aob_indirect_twice(
            "ChrFollowCam",
            &["48 8B 0D ? ? ? ? E8 ? ? ? ? 48 8B 4E 68 48 8B 05 ? ? ? ? 48 89 48 60"],
            3,
            7,
            true,
        ),
        aob_indirect_twice(
            "GraphicsData",
            &["48 8B 05 ? ? ? ? 48 8B 48 08 48 8B 01 48 8B 40 58"],
            3,
            7,
            true,
        ),
        aob_indirect_twice("AiTimer", &["48 8B 0D ? ? ? ? 48 85 C9 74 0E 48 83 C1 28"], 3, 7, true),
        // Static flag blocks. These are plain arrays of bytes in `.data`, not pointers,
        // so the chains that read them must not dereference.
        aob_indirect_twice(
            // The debug flag block (TKGP calls it ChrDbg): 19 booleans, one per flag,
            // `PlayerNoDead` at +0x0 through `PlayerReload` at +0x12.
            //
            // The pattern anchors on the routine that gates a stamina write on both the
            // per-character NoDamage bit and the global AllNoDamage flag, so the
            // displacement it carries resolves to ChrDbg+0x9. The trailing `10` instead
            // of the instruction length `19` subtracts that 0x9 back off, yielding the
            // base of the array.
            "ChrDbg",
            &["F6 81 24 05 00 00 40 48 8B D9 75 ? 80 3D ? ? ? ? 00"],
            14,
            10,
            true,
        ),
        aob_indirect_twice(
            // Five render toggles: map, objects, characters, SFX, cutscenes.
            "GroupMask",
            &["80 3D ? ? ? ? 00 BE 00 00 00 80"],
            2,
            7,
            true,
        ),
        // Functions. Calling these is markedly riskier than reading statics; keep them
        // behind a guard.
        aob_direct("GetEventFlagFn", &["40 53 48 83 EC 20 80 B9 24 02 00 00 00 8B DA 74 4D"], true),
        aob_direct(
            "SetEventFlagFn",
            &["48 89 5C 24 08 57 48 83 EC 20 80 B9 24 02 00 00 00 41 0F B6 F8"],
            true,
        ),
        aob_direct(
            "BonfireWarpFn",
            &["48 89 5C 24 08 57 48 83 EC 20 48 8B D9 8B FA 48 8B 49 08 48 85 C9 0F 84 ? ? ? ? \
               E8 ? ? ? ? 48 8B 4B 08"],
            true,
        ),
        aob_direct(
            "ItemGetFn",
            &["48 89 5C 24 18 89 54 24 10 55 56 57 41 54 41 55 41 56 41 57 48 8D 6C 24 F9"],
            true,
        ),
    ];

    let base_address_path = base_addresses_rs_path();
    let patches_path = patches_paths()?;
    codegen::codegen_base_addresses(base_address_path, patches_path, aobs);

    Ok(())
}
