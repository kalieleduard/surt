use std::io::{self, IsTerminal};

use windows_sys::Win32::System::Console::{
    CONSOLE_SCREEN_BUFFER_INFO, GetConsoleOutputCP, GetConsoleScreenBufferInfo, GetStdHandle,
    STD_ERROR_HANDLE,
};

const UTF8_CODE_PAGE: u32 = 65001;
const NO_CONSOLE: u32 = 0;

pub fn outputs_utf8() -> bool {
    let writes_to_console = io::stdout().is_terminal() || io::stderr().is_terminal();
    !writes_to_console || code_page_is_utf8(unsafe { GetConsoleOutputCP() })
}

pub(super) fn stderr_columns() -> Option<usize> {
    let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
    let found = unsafe { GetConsoleScreenBufferInfo(GetStdHandle(STD_ERROR_HANDLE), &mut info) };
    let columns = i32::from(info.srWindow.Right) - i32::from(info.srWindow.Left) + 1;
    usize::try_from(columns)
        .ok()
        .filter(|columns| found != 0 && *columns > 0)
}

pub(super) fn code_page_is_utf8(code_page: u32) -> bool {
    matches!(code_page, UTF8_CODE_PAGE | NO_CONSOLE)
}
