use core::fmt::Write;
use crate::devices::cga;
use crate::devices::lfb;
use crate::devices::pit;
use crate::devices::keyboard;
use crate::library::input;

pub extern "C" fn sys_get_system_time() -> u64 {
    pit::get_system_time() as u64
}

pub extern "C" fn sys_print(buffer: *const u8, len: usize) {
    let slice = unsafe { core::slice::from_raw_parts(buffer, len) };
    if let Ok(s) = core::str::from_utf8(slice) {
        if lfb::is_lfb_initialized() {
            buff_print!("{}", s);
        } else {
            let _ = cga::CGA.lock().write_str(s);
        }
    }
}

pub extern "C" fn sys_get_char() -> u64 {
    input::getch() as u64
}

pub extern "C" fn sys_set_color(color: u64) {
    crate::devices::buff_print::buff_set_color(color as u32);
}

pub extern "C" fn sys_buff_clear() {
    crate::devices::buff_print::buff_clear();
}

/// Returns (scancode << 8) | ascii. Blocks until a valid key is pressed.
pub extern "C" fn sys_get_key() -> u64 {
    loop {
        let key = keyboard::get_key_buffer().wait_for_key();
        if key.valid() {
            let scancode = key.get_scancode() as u64;
            let ascii = key.get_ascii() as u64;
            return (scancode << 8) | ascii;
        }
    }
}
