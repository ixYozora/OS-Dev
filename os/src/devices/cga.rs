
impl Write for CGA {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.print_byte(byte),

                // not part of printable ASCII range
                _ => self.print_byte(0xfe),
            }
        }

        Ok(())
    }
}
/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: cga                                                             ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: This module provides functions for doing output on the CGA text ║
   ║         screen. It also supports a text cursor position stored in the   ║
   ║         hardware using ports.                                           ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Michael Schoetter, Univ. Duesseldorf, 6.2.2024                  ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use spin::Mutex;
use crate::kernel::cpu as cpu;

/// Global CGA instance, used for screen output in the whole kernel.
/// Usage: let mut cga = cga::CGA.lock();
///        cga.print_byte(b'X');
pub static CGA: Mutex<CGA> = Mutex::new(CGA::new());

/// All 16 CGA colors.
#[repr(u8)] // store each enum variant as an u8
pub enum Color {
    Black      = 0,
    Blue       = 1,
    Green      = 2,
    Cyan       = 3,
    Red        = 4,
    Pink       = 5,
    Brown      = 6,
    LightGray  = 7,
    DarkGray   = 8,
    LightBlue  = 9,
    LightGreen = 10,
    LightCyan  = 11,
    LightRed   = 12,
    LightPink  = 13,
    Yellow     = 14,
    White      = 15,
}

pub const CGA_STD_ATTR: u8 = (Color::Black as u8) << 4 | (Color::Green as u8);

const CGA_BASE_ADDR: *mut u8 = 0xb8000 as *mut u8;
const CGA_ROWS: usize = 25;
const CGA_COLUMNS: usize = 80;

const CGA_INDEX_PORT: u16 = 0x3d4; // select register
const CGA_DATA_PORT: u16 = 0x3d5;  // read/write register
const CGA_HIGH_BYTE_CMD: u8 = 14;  // cursor high byte
const CGA_LOW_BYTE_CMD: u8 = 15;   // cursor high byte

pub struct CGA {
    index_port: cpu::IoPort,
    data_port: cpu::IoPort
}

impl CGA {
    /// Create a new CGA instance.
    const fn new() -> CGA {
        CGA {
            index_port: cpu::IoPort::new(CGA_INDEX_PORT),
            data_port: cpu::IoPort::new(CGA_DATA_PORT)
        }
    }

    /// Clear CGA screen and set cursor position to (0, 0).
    pub fn clear(&mut self) {
        /* Hier muss Code eingefuegt werden */
        for y in 0..CGA_ROWS {
            for x in 0..CGA_COLUMNS {
                let pos = (y * CGA_COLUMNS + x) * 2;
                unsafe {
                    CGA_BASE_ADDR.offset(pos as isize).write(b' ');
                    CGA_BASE_ADDR.offset((pos + 1) as isize).write(CGA_STD_ATTR);
                }
            }
        }
        self.setpos(0, 0);
    }

    /// Display the `character` at the given position `x`,`y` with attribute `attrib`.
    pub fn show(&mut self, x: usize, y: usize, character: char, attrib: u8) {
        if x > CGA_COLUMNS || y > CGA_ROWS {
            return;
        }

        let pos = (y * CGA_COLUMNS + x) * 2;

        // Write character and attribute to the screen buffer.
        //
        // Unsafe because we are writing directly to memory using a pointer.
        // We ensure that the pointer is valid by using CGA_BASE_ADDR
        // and checking the bounds of x and y.
        unsafe {
            CGA_BASE_ADDR.offset(pos as isize).write(character as u8);
            CGA_BASE_ADDR.offset((pos + 1) as isize).write(attrib);
        }
    }

    /// Return cursor position `x`,`y`
    pub fn getpos(&mut self) -> (usize, usize) {
        /* Hier muss Code eingefuegt werden */
        unsafe {
            self.index_port.outb(CGA_HIGH_BYTE_CMD);
        }
        let high = unsafe {
            self.data_port.inb()
        };
        // Select low byte
        unsafe {
            self.index_port.outb(CGA_LOW_BYTE_CMD);
        }
        let low = unsafe {
            self.data_port.inb()
        };

        let pos = ((high as u16) << 8) | (low as u16);
        let x = (pos as usize) % CGA_COLUMNS;
        let y = (pos as usize) / CGA_COLUMNS;
        (x, y) // Platzhalter, entfernen und durch sinnvollen Rueckgabewert ersetzen
    }

    /// Set cursor position `x`,`y`
    pub fn setpos(&mut self, x: usize, y: usize) {
        /* Hier muss Code eingefuegt werden */
        let pos = (y * CGA_COLUMNS + x) as u16;
        let high = (pos >> 8) as u8;
        let low = (pos & 0xFF) as u8;

        unsafe {
            self.index_port.outb(CGA_HIGH_BYTE_CMD);
            self.data_port.outb(high);
            self.index_port.outb(CGA_LOW_BYTE_CMD);
            self.data_port.outb(low);
        }
    }

    /// Print byte `b` at actual position cursor position `x`,`y`
    pub fn print_byte(&mut self, b: u8) {
        /* Hier muss Code eingefuegt werden */
        let (mut x, mut y) = self.getpos();

        match b {
            b'\n' => {
                x = 0;
                y += 1;
            }
            byte => {
                self.show(x, y, byte as char, CGA_STD_ATTR);
                x += 1;
                if x >= CGA_COLUMNS {
                    x = 0;
                    y += 1;
                }
            }
        }

        if y >= CGA_ROWS {
            self.scrollup();
            y = CGA_ROWS - 1;
        }

        self.setpos(x, y);
    }

    /// Scroll text lines by one to the top.
    pub fn scrollup(&mut self) {
        /* Hier muss Code eingefuegt werden */
        // Move each line up by one
        for y in 1..CGA_ROWS {
            for x in 0..CGA_COLUMNS {
                let from = ((y * CGA_COLUMNS + x) * 2) as isize;
                let to = (((y - 1) * CGA_COLUMNS + x) * 2) as isize;
                unsafe {
                    let ch = CGA_BASE_ADDR.offset(from).read();
                    let attr = CGA_BASE_ADDR.offset(from + 1).read();
                    CGA_BASE_ADDR.offset(to).write(ch);
                    CGA_BASE_ADDR.offset(to + 1).write(attr);
                }
            }
        }
        // Clear the last line
        for x in 0..CGA_COLUMNS {
            let pos = ((CGA_ROWS - 1) * CGA_COLUMNS + x) * 2;
            unsafe {
                CGA_BASE_ADDR.offset(pos as isize).write(b' ');
                CGA_BASE_ADDR.offset((pos + 1) as isize).write(CGA_STD_ATTR);
            }
        }
    }

    /// Helper function returning an attribute byte for the given parameters `bg`, `fg`, and `blink`
    /// Note: Blinking characters do not work in QEMU, but work on real hardware.
    ///       Support for blinking characters is optional and can be removed, if you want.
    pub fn attribute(&mut self, bg: Color, fg: Color, blink: bool) -> u8 {
        /* Hier muss Code eingefuegt werden */
        let mut attr = ((bg as u8) << 4) | (fg as u8);
        if blink {
            attr |= 0x80;
        }
        attr
    }
}