use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, STILL_ACTIVE};
use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE, TerminateProcess,
};

use super::Stop;

const BOOT_KEY: &str =
    r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management\PrefetchParameters";
const BOOT_VALUE: &str = "BootId";
const STOPPED_EXIT_CODE: u32 = 1;
const UNKNOWN_BOOT: &str = "unknown";
const HIGH_PART_SHIFT: u32 = 32;

struct Process(HANDLE);

impl Process {
    fn open(pid: u32, access: u32) -> Option<Process> {
        let handle = unsafe { OpenProcess(access, 0, pid) };
        (!handle.is_null()).then_some(Process(handle))
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

pub(super) fn start_time(pid: u32) -> Option<i64> {
    let process = Process::open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let mut exit_code = 0_u32;
    let still_active = u32::try_from(STILL_ACTIVE).ok()?;
    if unsafe { GetExitCodeProcess(process.0, &raw mut exit_code) } == 0
        || exit_code != still_active
    {
        return None;
    }
    let mut created = empty_time();
    let mut exited = empty_time();
    let mut kernel = empty_time();
    let mut user = empty_time();
    let read = unsafe {
        GetProcessTimes(
            process.0,
            &raw mut created,
            &raw mut exited,
            &raw mut kernel,
            &raw mut user,
        )
    };
    if read == 0 {
        return None;
    }
    let ticks =
        (u64::from(created.dwHighDateTime) << HIGH_PART_SHIFT) | u64::from(created.dwLowDateTime);
    i64::try_from(ticks).ok()
}

pub(super) fn boot_id() -> Option<String> {
    let key = wide(BOOT_KEY);
    let value = wide(BOOT_VALUE);
    let mut boot = 0_u32;
    let mut size = u32::try_from(size_of::<u32>()).ok()?;
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            ptr::null_mut(),
            (&raw mut boot).cast(),
            &raw mut size,
        )
    };
    Some(if read == 0 {
        boot.to_string()
    } else {
        UNKNOWN_BOOT.to_owned()
    })
}

pub(super) fn signal(pid: u32, _stop: Stop) {
    if let Some(process) = Process::open(pid, PROCESS_TERMINATE) {
        unsafe { TerminateProcess(process.0, STOPPED_EXIT_CODE) };
    }
}

fn empty_time() -> FILETIME {
    FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
