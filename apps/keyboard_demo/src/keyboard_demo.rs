#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{usr_print, usr_get_key, usr_thread_exit};

#[global_allocator]
static _A: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

const SC_ESC: u8 = 1;
const SC_ENTER: u8 = 28;
const SC_BACKSPACE: u8 = 14;

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_print("=== Keyboard Demo ===\n");
    usr_print("Type anything to see keyboard input.\n");
    usr_print("Press ESC to exit back to shell.\n");
    usr_print("Press BACKSPACE to delete characters.\n");
    usr_print("Press ENTER for new line.\n\n");

    loop {
        let raw = usr_get_key();
        let scancode = ((raw >> 8) & 0xFF) as u8;
        let ascii = (raw & 0xFF) as u8;
        match scancode {
            SC_ESC => {
                usr_print("\n\nKeyboard demo exited.\n");
                break;
            }
            SC_ENTER => {
                usr_print("\n");
            }
            SC_BACKSPACE => {
                usr_print("\x08");
            }
            _ => {
                if ascii != 0 && ascii < 0x80 {
                    let ch = ascii as char;
                    if ch.is_ascii_graphic() || ch == ' ' {
                        let s = [ascii];
                        if let Ok(t) = core::str::from_utf8(&s) {
                            usr_print(t);
                        }
                    }
                }
            }
        }
    }

    usr_thread_exit();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
