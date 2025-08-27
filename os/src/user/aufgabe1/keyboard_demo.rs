use crate::devices::cga as cga; // shortcut for cga
use crate::devices::cga_print; // used to import code needed by println!
use crate::devices::key as key; // shortcut for key
use crate::devices::keyboard as keyboard; // shortcut for keyboard
use crate::library::input;
use crate::devices::lfb::{color, get_lfb, HHU_GREEN, WHITE};
use crate::buff_print;
use crate::devices::buff_print::{buff_print, buff_clear, buff_set_color};

/// Clear any remaining keys from the keyboard buffer
fn clear_keyboard_buffer() {
    let key_buffer = keyboard::get_key_buffer();
    // Consume all remaining keys in the buffer
    while let Some(_) = key_buffer.get_last_key() {
        // Just consume and discard each key
    }
}

pub fn run() {
    // Print instructions for the user
    buff_print!("=== Keyboard Demo ===\n");
    buff_print!("Type anything to see keyboard input.\n");
    buff_print!("Press ESC to exit back to shell.\n");
    buff_print!("Press BACKSPACE to delete characters.\n");
    buff_print!("Press ENTER for new line.\n\n");

    loop {
        let key = keyboard::get_key_buffer().wait_for_key();
        if key.valid() {
            match key.get_scancode() {
                1 => { // ESC key
                    buff_print!("\n\nKeyboard demo exited.\n");
                    // Clear any remaining keys from the buffer to prevent carryover
                    clear_keyboard_buffer();
                    break; // Exit the demo
                }
                28 => { // Enter key
                    buff_print!("\n");
                }
                14 => { // Backspace
                    buff_print!("\x08"); // Send backspace to LFB printing
                }
                _ => {
                    let ascii = key.get_ascii();
                    if ascii != 0 && ascii.is_ascii() {
                        let ch = ascii as char;
                        if ch.is_ascii_graphic() || ch == ' ' {
                            buff_print!("{}", ch);
                        }
                    }
                }
            }
        }
    }
}