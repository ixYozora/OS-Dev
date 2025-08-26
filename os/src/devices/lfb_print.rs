use crate::devices::lfb::{BLACK, GREEN, get_lfb, is_lfb_initialized};
use crate::library::mutex::Mutex;
use core::fmt;
use core::fmt::Write;
use crate::devices::lfb::LFB;
use crate::lfb_print;

/// The global writer that can used as an interface from other modules.
/// It is threadsafe by using 'Mutex'.
pub static WRITER: Mutex<Writer> = Mutex::new(Writer::new());

/// Writer for writing formatted strings to the LFB screen
pub struct Writer {
    cursor_x: u32,
    cursor_y: u32,
    color: u32,
}

impl Writer {
    pub const fn new() -> Writer {
        Writer {
            cursor_x: 0,
            cursor_y: 0,
            color: GREEN,
        }
    }

    pub fn set_color(&mut self, color: u32) {
        self.color = color;
    }

    pub fn get_cursor_pos(&self) -> (u32, u32) {
        (self.cursor_x, self.cursor_y)
    }

    pub fn set_cursor_pos(&mut self, x: u32, y: u32) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    pub fn clear(&mut self) {
        if is_lfb_initialized() {
            if let Some(mut lfb) = get_lfb().try_lock() {
                lfb.clear();
                self.cursor_x = 0;
                self.cursor_y = 0;
            }
        }
    }

    fn write_char(&mut self, c: char, lfb: &mut LFB) {
        match c {
            '\n' => self.newline(lfb),
            '\r' => self.cursor_x = 0,
            '\t' => {
                for _ in 0..4 {
                    self.write_char(' ', lfb);
                }
            }
            '\x08' => self.backspace(lfb),
            c if c.is_control() => {},
            c => {
                let (char_width, char_height) = lfb.get_char_dimensions();
                let (screen_width, screen_height) = lfb.get_dimensions();

                if self.cursor_x + char_width > screen_width {
                    self.newline(lfb);
                }

                if self.cursor_y + char_height > screen_height {
                    lfb.scroll_up();
                    self.cursor_y -= char_height;
                }

                lfb.draw_char(self.cursor_x, self.cursor_y, self.color, c);
                self.cursor_x += char_width;
            }
        }
    }

    fn newline(&mut self, lfb: &mut LFB) {
        let (_, char_height) = lfb.get_char_dimensions();
        let (_, screen_height) = lfb.get_dimensions();

        self.cursor_x = 0;
        self.cursor_y += char_height;

        if self.cursor_y + char_height > screen_height {
            lfb.scroll_up();
            self.cursor_y -= char_height;
        }
    }

    fn backspace(&mut self, lfb: &mut LFB) {
        let (char_width, char_height) = lfb.get_char_dimensions();
        if self.cursor_x >= char_width {
            self.cursor_x -= char_width;
            // Clear the character
            for y in 0..char_height {
                for x in 0..char_width {
                    lfb.draw_pixel(self.cursor_x + x, self.cursor_y + y, crate::devices::lfb::BLACK);
                }
            }
        } else if self.cursor_y >= char_height {
            let (screen_width, _) = lfb.get_dimensions();
            self.cursor_y -= char_height;
            self.cursor_x = ((screen_width / char_width) - 1) * char_width;
        }
    }

}


/// Implementation of the 'core::fmt::Write' trait for our Writer.
/// Required to output formatted strings.
/// Requires only one function 'write_str'.
impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let mut lfb = crate::devices::lfb::get_lfb().lock(); // lock once
        for c in s.chars() {
            self.write_char(c, &mut lfb); // pass &mut LFB
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! lfb_print {
    ($($arg:tt)*) => ({
        $crate::devices::lfb_print::lfb_print(format_args!($($arg)*));
    });
}
#[macro_export]
macro_rules! lfb_println {
    () => (lfb_print!("\n"));
    ($fmt:expr) => (lfb_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (lfb_print!(concat!($fmt, "\n"), $($arg)*));
}

/// Helper function of print macros (must be public)
pub fn lfb_print(args: fmt::Arguments) {
    if is_lfb_initialized() {
        WRITER.lock().write_fmt(args).unwrap();
    }
}

/// Clear the LFB screen
pub fn lfb_clear() {
    if is_lfb_initialized() {
        WRITER.lock().clear();
    }
}

/// Set the text color for LFB output
pub fn lfb_set_color(color: u32) {
    if is_lfb_initialized() {
        WRITER.lock().set_color(color);
    }
}

/// Get current cursor position
pub fn lfb_get_cursor_pos() -> (u32, u32) {
    if is_lfb_initialized() {
        WRITER.lock().get_cursor_pos()
    } else {
        (0, 0)
    }
}

/// Set cursor position
pub fn lfb_set_cursor_pos(x: u32, y: u32) {
    if is_lfb_initialized() {
        WRITER.lock().set_cursor_pos(x, y);
    }
}

pub fn lfb_call_backspace(x: u32, y: u32, count: usize) {
    if is_lfb_initialized() {
        let mut writer = WRITER.lock();
        writer.set_cursor_pos(x, y);
        for _ in 0..count {
            let mut lfb = get_lfb().lock();
            writer.backspace(&mut lfb);
        }
    }


}
