use std::ffi::{CStr, c_void};
use std::mem::{MaybeUninit, size_of};

pub(super) use super::signals::signal;

const BOOT_SESSION: &CStr = c"kern.bootsessionuuid";
const UUID_BUFFER: usize = 64;
const MICROS_PER_SECOND: i64 = 1_000_000;

pub(super) fn start_time(pid: u32) -> Option<i64> {
    let pid = libc::c_int::try_from(pid).ok()?;
    let size = libc::c_int::try_from(size_of::<libc::proc_bsdinfo>()).ok()?;
    let mut info = MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    let written = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast::<c_void>(),
            size,
        )
    };
    if written != size {
        return None;
    }
    let info = unsafe { info.assume_init() };
    if info.pbi_status == libc::SZOMB {
        return None;
    }
    let seconds = i64::try_from(info.pbi_start_tvsec).ok()?;
    let micros = i64::try_from(info.pbi_start_tvusec).ok()?;
    Some(seconds * MICROS_PER_SECOND + micros)
}

pub(super) fn boot_id() -> Option<String> {
    let mut buffer = [0_u8; UUID_BUFFER];
    let mut length = buffer.len();
    let read = unsafe {
        libc::sysctlbyname(
            BOOT_SESSION.as_ptr(),
            buffer.as_mut_ptr().cast::<c_void>(),
            &raw mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if read != 0 {
        return None;
    }
    let id = CStr::from_bytes_until_nul(buffer.get(..length)?).ok()?;
    Some(id.to_str().ok()?.to_owned())
}
