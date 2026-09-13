use std::ops::{BitAnd, BitOr, BitXor, Not};
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Threading::GetCurrentProcess;

/// Master switch for every write this crate makes.
///
/// Reads are always safe: a bad chain evaluates to `None`. Writes are not, and
/// on a game build these addresses were not generated for, a write lands
/// somewhere arbitrary. Rather than guard each of the several dozen call sites,
/// the switch lives here, at the one place all of them funnel through.
static WRITES_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_writes_enabled(enabled: bool) {
    WRITES_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn writes_enabled() -> bool {
    WRITES_ENABLED.load(Ordering::Relaxed)
}

#[derive(Clone, Debug)]
pub struct PointerChain<T> {
    proc: HANDLE,
    base: *mut T,
    offsets: Vec<usize>,
}

unsafe impl<T> Send for PointerChain<T> {}
unsafe impl<T> Sync for PointerChain<T> {}

impl<T> PointerChain<T> {
    pub fn new(chain: &[usize]) -> PointerChain<T> {
        let mut it = chain.iter();
        let base = *it.next().unwrap() as *mut T;
        PointerChain {
            proc: unsafe { GetCurrentProcess() },
            base,
            offsets: it.copied().collect(), // it.map(|x| *x).collect(),
        }
    }

    fn safe_read(&self, addr: usize, offs: usize) -> Option<usize> {
        let mut value = 0usize;
        unsafe {
            ReadProcessMemory(
                self.proc,
                addr as _,
                &mut value as *mut usize as _,
                std::mem::size_of::<usize>(),
                None,
            )
            .ok()
            .map(|_| value + offs)
        }
    }

    /// Safely evaluates the pointer chain.
    /// Relies on `ReadProcessMemory` instead of pointer dereferencing for crash
    /// safety.  Returns `None` if the evaluation failed.
    pub fn eval(&self) -> Option<*mut T> {
        self.offsets
            .iter()
            .try_fold(self.base as usize, |addr, &offs| self.safe_read(addr, offs))
            .map(|addr| addr as *mut T)
    }

    pub fn read(&self) -> Option<T> {
        let ptr = self.eval()?;
        let mut value: T = unsafe { std::mem::zeroed() };
        unsafe {
            ReadProcessMemory(
                self.proc,
                ptr as _,
                &mut value as *mut _ as _,
                std::mem::size_of::<T>(),
                None,
            )
            .ok()
            .map(|_| value)
        }
    }

    /// Evaluates the pointer chain and attempts to write the datum.
    /// Returns `None` if writes are disabled, or if either the evaluation or
    /// the write failed.
    pub fn write(&self, mut value: T) -> Option<()> {
        if !writes_enabled() {
            return None;
        }

        let ptr = self.eval()?;
        unsafe {
            WriteProcessMemory(
                self.proc,
                ptr as _,
                &mut value as *mut _ as _,
                std::mem::size_of::<T>(),
                None,
            )
            .ok()
            .map(|_| ())
        }
    }
}

#[derive(Clone, Debug)]
pub struct Bitflag<T>(PointerChain<T>, T);

impl<T> Bitflag<T>
where
    T: BitXor<Output = T>
        + BitAnd<Output = T>
        + BitOr<Output = T>
        + Not<Output = T>
        + PartialEq
        + Copy,
{
    pub fn new(c: PointerChain<T>, mask: T) -> Self {
        Bitflag(c, mask)
    }

    pub fn toggle(&self) {
        if let Some(x) = self.0.read() {
            self.0.write(x ^ self.1);
        }
    }

    pub fn get(&self) -> Option<bool> {
        self.0.read().map(|x| (x & self.1) == self.1)
    }

    pub fn set(&self, flag: bool) {
        if let Some(x) = self.0.read() {
            self.0.write(if flag { x | self.1 } else { x & !self.1 });
        }
    }
}

#[macro_export]
macro_rules! pointer_chain {
    ($($e:expr),+) => { PointerChain::new(&[$($e,)*]) }
}

#[macro_export]
macro_rules! bitflag {
    ($b:expr; $($e:expr),+) => { Bitflag::new(PointerChain::new(&[$($e,)*]), $b) }
}

pub use {bitflag, pointer_chain};
