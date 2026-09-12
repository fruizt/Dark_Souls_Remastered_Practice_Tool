use std::ffi::c_void;
use std::mem::{self, size_of};
use std::path::PathBuf;

use windows::core::{s, w, Error, Result, HRESULT, HSTRING};
use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE, MAX_PATH};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    CreateRemoteThread, GetExitCodeThread, OpenProcess, WaitForSingleObject, INFINITE,
    PROCESS_ALL_ACCESS,
};

/// A process, open with the permissions appropriate for injection.
pub struct Process(HANDLE);

impl Process {
    /// Retrieve the process ID by executable name, returning the first match,
    /// and open it with the appropriate permissions.
    pub fn get_process_by_name(name: &str) -> Result<Self> {
        unsafe { get_process_by_name64(name).map(Self) }
    }

    /// Inject the DLL in the process.
    pub fn inject(&self, dll_path: PathBuf) -> Result<()> {
        let proc_addr =
            unsafe { GetProcAddress(GetModuleHandleW(w!("Kernel32"))?, s!("LoadLibraryW")) };
        let dll_path = HSTRING::from(dll_path.canonicalize().unwrap().as_path());
        let dll_path_buf = unsafe {
            VirtualAllocEx(
                self.0,
                None,
                (MAX_PATH as usize) * size_of::<u16>(),
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            )
        };

        let mut bytes_written = 0usize;
        let res = unsafe {
            WriteProcessMemory(
                self.0,
                dll_path_buf,
                dll_path.as_ptr() as *const c_void,
                (MAX_PATH as usize) * size_of::<u16>(),
                Some(&mut bytes_written),
            )
        };

        let thread = unsafe {
            CreateRemoteThread(
                self.0,
                None,
                0,
                proc_addr.map(|proc_addr| {
                    mem::transmute::<
                        unsafe extern "system" fn() -> isize,
                        unsafe extern "system" fn(*mut c_void) -> u32,
                    >(proc_addr)
                }),
                Some(dll_path_buf),
                0,
                None,
            )
        }?;

        unsafe {
            WaitForSingleObject(thread, INFINITE);
            let mut exit_code = 0u32;
            GetExitCodeThread(thread, &mut exit_code as *mut u32)?;
            CloseHandle(thread)?;
            VirtualFreeEx(self.0, dll_path_buf, 0, MEM_RELEASE)?;

            Ok(())
        }
    }

    /// Retrieve the process handle.
    pub fn handle(&self) -> HANDLE {
        self.0
    }
}

unsafe fn get_process_by_name64(name_str: &str) -> Result<HANDLE> {
    let name = HSTRING::from(name_str);

    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
    let mut process_entry32 =
        PROCESSENTRY32W { dwSize: mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };

    if Process32FirstW(snapshot, &mut process_entry32).is_err() {
        CloseHandle(snapshot)?;
        return Err(Error::from_win32());
    }

    let pid = loop {
        let zero_idx = process_entry32
            .szExeFile
            .iter()
            .position(|&x| x == 0)
            .unwrap_or(process_entry32.szExeFile.len());
        let process_name = HSTRING::from_wide(&process_entry32.szExeFile[..zero_idx])?;

        if name == process_name {
            break Ok(process_entry32.th32ProcessID);
        }

        if Process32NextW(snapshot, &mut process_entry32).is_err() {
            CloseHandle(snapshot)?;
            break Err(Error::from_hresult(HRESULT(-1)));
        }
    }?;

    CloseHandle(snapshot)?;

    OpenProcess(PROCESS_ALL_ACCESS, BOOL(0), pid)
}
