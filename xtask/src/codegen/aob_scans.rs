use std::env;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

// use practice_tool_tasks::codegen::{self, aob_direct, aob_indirect, aob_indirect_twice};
use super::codegen::{self, aob_indirect_twice};

/// Every `DarkSoulsRemastered.exe` found one level below `DSR_INSTALLS_DIR`.
///
/// The variable points at a directory that *contains* game installs (a Steam
/// library's `steamapps/common`, say), not at an install itself, so that several
/// game versions sitting side by side can be scanned in one run.
fn patches_paths() -> Result<impl Iterator<Item = PathBuf>> {
    let base_path = env::var("DSR_INSTALLS_DIR").map(PathBuf::from).context(
        "DSR_INSTALLS_DIR is not set. Copy .env.example to .env and point it at the directory \
         containing your Dark Souls Remastered install.",
    )?;

    if !base_path.is_dir() {
        bail!(
            "DSR_INSTALLS_DIR does not point at a directory: {}",
            base_path.display()
        );
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
    let aobs = &[
        aob_indirect_twice("BaseA", &["48 89 05 xx xx xx xx 8D 42"], 3, 7, true),
        aob_indirect_twice(
            "WorldChrMan",
            &["48 8B 05 xx xx xx xx 45 33 ED 48 8B F1 48 85 C0"],
            3,
            7,
            true,
        ),
        aob_indirect_twice(
            "CharacterFlags",
            &["48 8B 05 xx xx xx xx 48 39 48 68 0F 94 C0 C3"],
            3,
            7,
            true,
        ),
        aob_indirect_twice(
            "BaseMenu",
            &["48 8B 05 xx xx xx xx 48 63 C9 89 54 88 30"],
            3,
            7,
            true,
        ),
        aob_indirect_twice(
            "WorldChrDebug",
            &["48 8B 05 ? ? ? ? 48 8B 80 F0 00 00 00 48 85 C0"],
            3,
            7,
            true,
        ),
        // aob_indirect_twice(
        //     "MenuManBase",
        //     &["48 8B 05 ? ? ? ? 89 88 28 08 00 00 85 C9"],
        //     3,
        //     7,
        //     true,
        // ),
    ];

    let base_address_path = base_addresses_rs_path();
    let patches_path = patches_paths()?;
    codegen::codegen_base_addresses(base_address_path, patches_path, aobs);

    Ok(())
}
