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
use core::fmt::Write;
//use spin::Mutex;
use crate::library::mutex::Mutex;
//use crate::library::spinlock::Spinlock as Mutex;
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
pub(crate) const CGA_ROWS: usize = 25;
pub(crate) const CGA_COLUMNS: usize = 80;

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
        for y in 0..CGA_ROWS {
            for x in 0..CGA_COLUMNS {
                self.show(x, y, 0 as char, CGA_STD_ATTR);
            }
        }
        self.setpos(0,0);
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

        let high;
        let low;
        unsafe {
            self.index_port.outb(CGA_HIGH_BYTE_CMD);
            high = self.data_port.inb();
            self.index_port.outb(CGA_LOW_BYTE_CMD);
            low = self.data_port.inb();
        }
        //kprintln!("high : {}, low : {}", high, low);
        let pos = (high as u16) << 8 | (low as u16);
        //kprintln!("{}", pos);
        let y = (pos as usize) / CGA_COLUMNS;
        //let x = (pos as usize) % CGA_COLUMNS;
        let x = (pos as usize) - y* CGA_COLUMNS;



        //kprintln!("x: {}, y: {}", x, y);

        (x, y)
    }

    /// Set cursor position `x`,`y`
    pub fn setpos(&mut self, x: usize, y: usize) {
        // if x > CGA_COLUMNS || y > CGA_ROWS {
        //     return;
        // }

        let pos = y * CGA_COLUMNS + x;

        unsafe {
            self.index_port.outb(CGA_LOW_BYTE_CMD);
            self.data_port.outb((pos & 0xff) as u8);
            self.index_port.outb(CGA_HIGH_BYTE_CMD);
            self.data_port.outb(((pos >> 8) & 0xff) as u8);
        }
    }

    /// Print byte `b` at actual position cursor position `x`,`y`
    /// If byte is '\n' then we set cursor to new line
    /// Before trying to print outside the screen we scroll up one line
    pub fn print_byte(&mut self, b: u8) {
        let pos = self.getpos();

        if b != b'\n' {
            self.show(pos.0, pos.1, b as char, CGA_STD_ATTR);
        }

        if b == b'\n' || pos.0+1 >= CGA_COLUMNS  {
            self.setpos(0, pos.1+1);
            if pos.1+1 >= CGA_ROWS {
                self.scrollup();
            }
            return;
        }

        self.setpos(pos.0 + 1, pos.1);

    }

    /// Scroll text lines by one to the top.
    pub fn scrollup(&mut self) {

        for y in 0..CGA_ROWS-1 {
            for x in 0..CGA_COLUMNS {
                let pos_old = (y * CGA_COLUMNS + x) * 2;
                let pos_new = ((y+1) * CGA_COLUMNS + x) * 2;
                unsafe {
                    let new_char = CGA_BASE_ADDR.offset(pos_new as isize).read();
                    let new_attr = CGA_BASE_ADDR.offset((pos_new+1) as isize).read();
                    CGA_BASE_ADDR.offset(pos_old as isize).write(new_char);
                    CGA_BASE_ADDR.offset((pos_old+1) as isize).write(new_attr);
                }
            }
        }

        //Clear last row
        for x in 0..CGA_COLUMNS {
            self.show(x, CGA_ROWS-1, 0 as char, CGA_STD_ATTR);
        }

        let pos = self.getpos();
        self.setpos(pos.0, pos.1 - 1);
    }

    /// Helper function returning an attribute byte for the given parameters `bg`, `fg`, and `blink`
    /// Note: Blinking characters do not work in QEMU, but work on real hardware.
    ///       Support for blinking characters is optional and can be removed, if you want.
    pub fn attribute(&mut self, bg: Color, fg: Color, blink: bool) -> u8 {
        //let res = (blink as u8) << 8 | (bg as u8) << 4 | (fg as u8);
        (blink as u8) << 7 | (bg as u8) << 4 | (fg as u8)
    }

    /// deletes a char in fn
    pub fn del(&mut self) {
        let pos = self.getpos();
        if pos.0 <= 0 {
            self.setpos(CGA_COLUMNS-1, pos.1-1);
            self.show(CGA_COLUMNS-1, pos.1-1, 0 as char, CGA_STD_ATTR);
            return;
        }
        self.setpos(pos.0-1, pos.1);
        self.show(pos.0-1, pos.1, 0 as char, CGA_STD_ATTR);
    }
}

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