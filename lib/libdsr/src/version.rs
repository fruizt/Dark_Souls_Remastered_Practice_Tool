//! Which build of the game the tool is attached to.
//!
//! Every address in this crate was generated against a specific executable, so
//! attaching to a different one means reading and writing memory that belongs
//! to something else. That is worth detecting, and worth refusing to write on.
//!
//! Detection reads the mapped size of the running module out of its own PE
//! headers. It is not the prettiest signal — the file version resource would
//! read better — but it needs no extra Windows APIs, it is distinct per patch
//! (DSR-Gadget used the same one to tell four builds apart), and `xtask`
//! records it for every executable it scans, so the table cannot drift from the
//! addresses beside it.

use once_cell::sync::Lazy;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;

use crate::codegen::base_addresses::Version;
use crate::memedit::{set_writes_enabled, PointerChain};

/// What the tool found when it looked at the module it is running inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionState {
    Supported(Version),
    /// A real module, of a size no generated address set matches.
    Unsupported {
        size_of_image: usize,
    },
    /// The PE headers did not look like PE headers.
    Unreadable,
}

impl VersionState {
    pub fn version(self) -> Option<Version> {
        match self {
            VersionState::Supported(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_supported(self) -> bool {
        matches!(self, VersionState::Supported(_))
    }
}

/// Detected once, on first use. Writes are switched off here rather than at
/// every call site, so an unsupported build cannot corrupt a save through a
/// widget nobody remembered to guard.
pub static VERSION_STATE: Lazy<VersionState> = Lazy::new(|| {
    let state = detect();

    match state {
        // Logged on the way through, not just on failure: when a future patch
        // breaks something, the first question is which build the tool thought
        // it was on, and the log should already answer it.
        VersionState::Supported(version) => {
            let (maj, min, patch) = version.into();
            log::info!(
                "Detected game version {maj}.{min:02}.{patch} (module {:#x})",
                module_size().unwrap_or(0)
            );
        },
        VersionState::Unsupported { size_of_image } => {
            log::error!(
                "Unsupported game version: module is {size_of_image:#x} bytes, which matches none \
                 of {:?}. Writes are disabled.",
                Version::KNOWN.iter().map(|(s, _)| *s).collect::<Vec<_>>()
            );
            set_writes_enabled(false);
        },
        VersionState::Unreadable => {
            log::error!("Couldn't read the game's PE headers. Writes are disabled.");
            set_writes_enabled(false);
        },
    }

    state
});

/// The version whose addresses are in use.
///
/// On an unsupported build this falls back to the newest one known so the rest
/// of the tool still has offsets to talk about — but [`VERSION_STATE`] has
/// already disabled writes by then, so nothing acts on them.
pub static VERSION: Lazy<Version> =
    Lazy::new(|| VERSION_STATE.version().unwrap_or(Version::KNOWN[Version::KNOWN.len() - 1].1));

pub fn is_supported() -> bool {
    VERSION_STATE.is_supported()
}

fn detect() -> VersionState {
    let Some(size) = module_size() else {
        return VersionState::Unreadable;
    };

    match Version::from_module_size(size) {
        Some(version) => VersionState::Supported(version),
        None => VersionState::Unsupported { size_of_image: size },
    }
}

/// `SizeOfImage` from the headers of the module we are loaded into.
///
/// Read through [`PointerChain`] like everything else here: these headers are
/// always mapped, but a wrong offset in a header we do not control should still
/// come back as `None` rather than a fault.
fn module_size() -> Option<usize> {
    let base = unsafe { GetModuleHandleA(None) }.ok()?.0 as usize;
    if base == 0 {
        return None;
    }

    // IMAGE_DOS_HEADER::e_lfanew, then the PE signature it points at.
    let e_lfanew = PointerChain::<u32>::new(&[base + 0x3c]).read()? as usize;
    let nt = base + e_lfanew;

    if PointerChain::<u32>::new(&[nt]).read()? != 0x0000_4550 {
        return None;
    }

    // IMAGE_OPTIONAL_HEADER64 starts at +0x18; SizeOfImage sits 0x38 into it.
    if PointerChain::<u16>::new(&[nt + 0x18]).read()? != 0x20b {
        return None;
    }

    PointerChain::<u32>::new(&[nt + 0x18 + 0x38]).read().map(|size| size as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_generated_version_is_reachable() {
        assert!(!Version::KNOWN.is_empty(), "xtask generated no versions");

        for &(size, version) in Version::KNOWN {
            assert_eq!(Version::from_module_size(size), Some(version));
        }
    }

    #[test]
    fn an_unknown_size_is_not_a_version() {
        assert_eq!(Version::from_module_size(0x1234), None);
    }

    /// The fallback exists so the rest of the tool has offsets to name on an
    /// unsupported build; it must never be mistaken for support.
    #[test]
    fn unsupported_states_report_no_version() {
        assert_eq!(VersionState::Unsupported { size_of_image: 0x1234 }.version(), None);
        assert!(!VersionState::Unsupported { size_of_image: 0x1234 }.is_supported());
        assert!(!VersionState::Unreadable.is_supported());
    }
}
