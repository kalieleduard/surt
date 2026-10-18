use std::io::{self, IsTerminal};

use windows_sys::Win32::System::Console::GetConsoleOutputCP;

const UTF8_CODE_PAGE: u32 = 65001;
const NO_CONSOLE: u32 = 0;

pub fn outputs_utf8() -> bool {
    let writes_to_console = io::stdout().is_terminal() || io::stderr().is_terminal();
    !writes_to_console || code_page_is_utf8(unsafe { GetConsoleOutputCP() })
}

pub(super) fn code_page_is_utf8(code_page: u32) -> bool {
    matches!(code_page, UTF8_CODE_PAGE | NO_CONSOLE)
}
