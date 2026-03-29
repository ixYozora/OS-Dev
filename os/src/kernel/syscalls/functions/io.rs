use core::fmt::Write;
use crate::devices::cga;
use crate::devices::buff_print::WRITER;
use crate::devices::lfb;
use crate::devices::pcspk;
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

/// PC speaker from Ring 3. `tune_id`: 0 = tetris, 1 = aerodynamic, 2 = both in sequence.
pub extern "C" fn sys_pcspk_play(tune_id: u64) -> u64 {
    match tune_id {
        0 => pcspk::tetris(),
        1 => pcspk::aerodynamic(),
        2 => {
            pcspk::tetris();
            pcspk::aerodynamic();
        }
        _ => {}
    }
    0
}

/// Packed `(cursor_x << 32) | cursor_y` from the LFB text writer.
pub extern "C" fn sys_get_text_cursor() -> u64 {
    if !lfb::is_lfb_initialized() {
        return 0;
    }
    let w = WRITER.lock();
    let (x, y) = w.get_cursor_pos();
    ((x as u64) << 32) | (y as u64)
}

pub extern "C" fn sys_set_text_cursor(x: u64, y: u64) -> u64 {
    if !lfb::is_lfb_initialized() {
        return 0;
    }
    let mut w = WRITER.lock();
    w.set_cursor_pos(x as u32, y as u32);
    0
}

/// Clear `count` horizontal text bands starting at `base_y`, each step `step_px` down (legacy uses 16).
pub extern "C" fn sys_clear_text_bands(base_y: u64, step_px: u64, count: u64) -> u64 {
    if !lfb::is_lfb_initialized() {
        return 0;
    }
    let count = count.min(64);
    let mut g = lfb::get_lfb().lock();
    let mut y = base_y as u32;
    for _ in 0..count {
        g.clear_text_line(y);
        y = y.saturating_add(step_px as u32);
    }
    0
}
