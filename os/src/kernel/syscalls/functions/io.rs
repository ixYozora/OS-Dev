use core::fmt::Write;
use crate::devices::cga;
use crate::devices::pit;
use crate::library::input;

pub extern "C" fn sys_get_system_time() -> u64 {
    pit::get_system_time() as u64
}

pub extern "C" fn sys_print(buffer: *const u8, len: usize) {
    let slice = unsafe { core::slice::from_raw_parts(buffer, len) };
    if let Ok(s) = core::str::from_utf8(slice) {
        let _ = cga::CGA.lock().write_str(s);
    }
}

pub extern "C" fn sys_get_char() -> u64 {
    input::getch() as u64
}
