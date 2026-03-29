#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{usr_print, usr_thread_exit, usr_thread_get_id, usr_thread_yield};

#[global_allocator]
static _A: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_print("=== Thread yield demo (single user process) ===\n");
    usr_print("Three logical counters rotate in one thread; usr_thread_yield() gives the CPU to other processes.\n\n");

    let tid = usr_thread_get_id();
    let mut c0: u32 = 0;
    let mut c1: u32 = 0;
    let mut c2: u32 = 0;
    let mut which: u8 = 0;

    for iter in 0..30_000 {
        match which {
            0 => {
                c0 = c0.wrapping_add(1);
                which = 1;
            }
            1 => {
                c1 = c1.wrapping_add(1);
                which = 2;
            }
            _ => {
                c2 = c2.wrapping_add(1);
                which = 0;
            }
        }
        if iter % 1500 == 0 {
            usr_print("TID ");
            print_usize(tid);
            usr_print("  [0] ");
            print_u32(c0);
            usr_print("  [1] ");
            print_u32(c1);
            usr_print("  [2] ");
            print_u32(c2);
            usr_print("\n");
        }
        usr_thread_yield();
    }

    usr_print("Demo finished.\n");
    usr_thread_exit();
}

fn print_u32(v: u32) {
    let mut buf = [0u8; 12];
    let n = write_u32(v, &mut buf);
    if let Ok(s) = core::str::from_utf8(&buf[..n]) {
        usr_print(s);
    }
}

fn print_usize(v: usize) {
    let mut buf = [0u8; 20];
    let n = write_usize(v, &mut buf);
    if let Ok(s) = core::str::from_utf8(&buf[..n]) {
        usr_print(s);
    }
}

fn write_u32(mut v: u32, buf: &mut [u8]) -> usize {
    if v == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut i = 0;
    while v > 0 {
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        i += 1;
    }
    for j in 0..i / 2 {
        buf.swap(j, i - 1 - j);
    }
    i
}

fn write_usize(mut v: usize, buf: &mut [u8]) -> usize {
    if v == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut i = 0;
    while v > 0 {
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        i += 1;
    }
    for j in 0..i / 2 {
        buf.swap(j, i - 1 - j);
    }
    i
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
