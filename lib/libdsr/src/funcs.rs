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

use crate::memedit::{writes_enabled, PointerChain};

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
        if !writes_enabled() {
            return false;
        }

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

/// Put an item straight into your inventory.
///
/// Unlike [`BonfireWarp`], this one is not a plain call: the routine takes
/// eight arguments, four of them bytes on the stack, and it branches on all
/// four.
///
/// The stub below is modelled on the game's own call site at `0x4DD5B5`, which
/// passes every one of those four as a constant. An earlier version copied
/// DSR-Gadget's stub instead, which reproduces a call site's *instructions*
/// without its *frame*: it writes the four bytes at `[rsp+0x20..0x38]` and only
/// then does `sub rsp, 0x38`, which leaves them 0x38 away from where the callee
/// reads them. The call then ran with whatever was on the fresh thread's stack,
/// took the "don't grant it" path, and returned quietly. Allocating the frame
/// first is the fix.
///
/// The stub clobbers r14 and r15 without saving them, so it must be entered as
/// a thread rather than called as a function.
#[derive(Debug, Clone)]
pub struct ItemSpawn {
    game_data_man: usize,
    item_get_fn: usize,
}

/// 0x48 of frame: 0x20 of shadow space, 0x20 for the four stack arguments, and
/// 8 more to land the `call` on a 16 byte boundary from a thread entry.
#[rustfmt::skip]
const ITEM_SPAWN_STUB: [u8; 0x50] = [
    0x48, 0x83, 0xEC, 0x48,                                           // sub  rsp, 0x48
    0xBA, 0xFE, 0xFE, 0xFE, 0xFE,                                     // mov  edx, category
    0x41, 0xB9, 0xFE, 0xFE, 0xFE, 0xFE,                               // mov  r9d, quantity
    0x41, 0xB8, 0xFE, 0xFE, 0xFE, 0xFE,                               // mov  r8d, item id
    0x48, 0xA1, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,       // mov  rax, [GameDataMan]
    0xC6, 0x44, 0x24, 0x20, 0x01,                                     // mov  byte [rsp+0x20], 1
    0xC6, 0x44, 0x24, 0x28, 0x01,                                     // mov  byte [rsp+0x28], 1
    0xC6, 0x44, 0x24, 0x30, 0x00,                                     // mov  byte [rsp+0x30], 0
    0xC6, 0x44, 0x24, 0x38, 0x01,                                     // mov  byte [rsp+0x38], 1
    0x4C, 0x8B, 0x78, 0x10,                                           // mov  r15, [rax+0x10]
    0x49, 0x8D, 0x8F, 0x80, 0x02, 0x00, 0x00,                         // lea  rcx, [r15+0x280]
    0x49, 0xBE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,       // mov  r14, ItemGetFn
    0x41, 0xFF, 0xD6,                                                 // call r14
    0x48, 0x83, 0xC4, 0x48,                                           // add  rsp, 0x48
    0xC3,                                                             // ret
];

const STUB_CATEGORY: usize = 0x05;
const STUB_QUANTITY: usize = 0x0B;
const STUB_ITEM_ID: usize = 0x11;
const STUB_GAME_DATA_MAN: usize = 0x17;
const STUB_ITEM_GET_FN: usize = 0x40;

impl ItemSpawn {
    pub fn new(game_data_man: usize, item_get_fn: usize) -> Self {
        Self { game_data_man, item_get_fn }
    }

    pub fn is_ready(&self) -> bool {
        PointerChain::<usize>::new(&[self.game_data_man]).read().is_some_and(|p| p != 0)
    }

    /// `category` is the item's high nibble — 0x00000000 weapons, 0x10000000
    /// armour, 0x20000000 rings, 0x40000000 goods — and `item_id` the rest of
    /// it.
    ///
    /// Returns false without calling anything if there is no character loaded.
    pub fn spawn(&self, category: u32, item_id: u32, quantity: u32) -> bool {
        if !writes_enabled() || !self.is_ready() {
            return false;
        }

        let mut stub = ITEM_SPAWN_STUB;
        stub[STUB_CATEGORY..][..4].copy_from_slice(&category.to_le_bytes());
        stub[STUB_QUANTITY..][..4].copy_from_slice(&quantity.to_le_bytes());
        stub[STUB_ITEM_ID..][..4].copy_from_slice(&item_id.to_le_bytes());
        stub[STUB_GAME_DATA_MAN..][..8].copy_from_slice(&(self.game_data_man as u64).to_le_bytes());
        stub[STUB_ITEM_GET_FN..][..8].copy_from_slice(&(self.item_get_fn as u64).to_le_bytes());

        thread::spawn(move || unsafe { run_stub(&stub) });

        true
    }
}

/// Copy `stub` into executable memory, run it as a thread, and clean up after
/// it.
///
/// # Safety
///
/// `stub` must be a complete, correctly patched function that ends in `ret`.
unsafe fn run_stub(stub: &[u8]) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Memory::{
        VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
    };
    use windows::Win32::System::Threading::{
        CreateThread, WaitForSingleObject, THREAD_CREATION_FLAGS,
    };

    let mem = VirtualAlloc(None, stub.len(), MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    if mem.is_null() {
        return;
    }

    std::ptr::copy_nonoverlapping(stub.as_ptr(), mem as *mut u8, stub.len());

    let entry: extern "system" fn(*mut std::ffi::c_void) -> u32 = std::mem::transmute(mem);

    match CreateThread(None, 0, Some(entry), None, THREAD_CREATION_FLAGS(0), None) {
        Ok(thread) => {
            // Bounded, so a call that never returns leaks a page instead of
            // wedging this thread forever.
            WaitForSingleObject(thread, 5000);
            CloseHandle(thread).ok();
        },
        Err(e) => log::error!("Couldn't run item spawn stub: {e}"),
    }

    VirtualFree(mem, 0, MEM_RELEASE).ok();
}
