#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{usr_print, usr_thread_exit};

#[global_allocator]
static _A: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_print("Test der Zahlenausgabenfunktion:\n\n");
    usr_print("dex    hex   bin\n");

    for i in 0..=16u32 {
        let mut line = [0u8; 48];
        let n = format_row(i, &mut line);
        usr_print(unsafe { core::str::from_utf8_unchecked(&line[..n]) });
    }

    usr_print("Demo finished.\n");
    usr_thread_exit();
}

fn format_row(i: u32, buf: &mut [u8]) -> usize {
    let mut pos = 0;
    pos += write_padded_u32(i, 3, &mut buf[pos..]);
    buf[pos] = b' ';
    pos += 1;
    buf[pos] = b' ';
    pos += 1;
    buf[pos] = b' ';
    pos += 1;
    buf[pos..pos + 2].copy_from_slice(b"0x");
    pos += 2;
    pos += write_hex_u8(i as u8, &mut buf[pos..]);
    buf[pos] = b' ';
    pos += 1;
    buf[pos] = b' ';
    pos += 1;
    buf[pos] = b' ';
    pos += 1;
    pos += write_bin_u8(i as u8, &mut buf[pos..]);
    buf[pos] = b'\n';
    pos + 1
}

fn write_padded_u32(mut v: u32, width: usize, out: &mut [u8]) -> usize {
    let mut tmp = [0u8; 10];
    let mut n = 0;
    if v == 0 {
        tmp[0] = b'0';
        n = 1;
    } else {
        while v > 0 {
            tmp[n] = b'0' + (v % 10) as u8;
            v /= 10;
            n += 1;
        }
    }
    let mut pos = 0;
    if width > n {
        for _ in 0..(width - n) {
            out[pos] = b' ';
            pos += 1;
        }
    }
    for i in 0..n {
        out[pos + i] = tmp[n - 1 - i];
    }
    pos + n
}

fn write_hex_u8(v: u8, out: &mut [u8]) -> usize {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out[0] = HEX[(v >> 4) as usize];
    out[1] = HEX[(v & 0xf) as usize];
    2
}

fn write_bin_u8(v: u8, out: &mut [u8]) -> usize {
    let bits = if v == 0 {
        1
    } else {
        8 - v.leading_zeros() as usize
    };
    let width = core::cmp::max(4, bits);
    for b in 0..width {
        let shift = width - 1 - b;
        out[b] = if (v >> shift) & 1 != 0 { b'1' } else { b'0' };
    }
    width
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
