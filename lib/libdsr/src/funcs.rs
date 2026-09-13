//! Calls into the game's own code.
//!
//! Everything else in this crate reads and writes memory, which is recoverable:
//! a bad pointer chain evaluates to `None` and the UI shrugs. Calling a
//! function is not recoverable — a wrong address or a wrong signature crashes
//! the game. So the rules here are stricter than elsewhere:
//!
//! - Resolve every pointer through `ReadProcessMemory` first and refuse to call
//!   if anything comes back `None`. No world loaded means no call.
//! - Run the call on its own thread rather than on the render thread the
//!   overlay draws from, mirroring what DSR-Gadget does with a remote thread.

use std::thread;

use crate::memedit::PointerChain;

/// Warp to the bonfire currently recorded as your last one.
///
/// The destination is whatever sits in `ChrClassWarp`'s last-bonfire field, so
/// setting that first is what chooses where you land.
#[derive(Debug, Clone)]
pub struct BonfireWarp {
    /// Static holding the `GameDataMan` pointer.
    game_data_man: usize,
    warp_fn: usize,
}

impl BonfireWarp {
    pub fn new(game_data_man: usize, warp_fn: usize) -> Self {
        Self { game_data_man, warp_fn }
    }

    /// The instance the call needs, or `None` on the main menu.
    fn instance(&self) -> Option<usize> {
        let ptr = PointerChain::<usize>::new(&[self.game_data_man]).read()?;
        (ptr != 0).then_some(ptr)
    }

    /// Whether a call would have something to act on. Drives the disabled state
    /// of the button, so it is cheap and called every frame.
    pub fn is_ready(&self) -> bool {
        self.instance().is_some()
    }

    /// Returns false if there was nothing to call with; the call itself is fire
    /// and forget, since the game tears down the map underneath it.
    pub fn warp(&self) -> bool {
        let Some(instance) = self.instance() else {
            return false;
        };

        let warp_fn = self.warp_fn;

        // Off the render thread: this call does not return until the game has
        // begun unloading the map, and blocking Present through that is asking
        // for a deadlock.
        thread::spawn(move || {
            // SAFETY: `warp_fn` came from an AOB scan of the running module and
            // `instance` was read back from the game moments ago. Neither can be
            // proven correct from here — this is the crash-prone edge of the tool.
            unsafe {
                let f: extern "C" fn(usize, u32) -> u32 = std::mem::transmute(warp_fn);
                f(instance, 1);
            }
        });

        true
    }
}
