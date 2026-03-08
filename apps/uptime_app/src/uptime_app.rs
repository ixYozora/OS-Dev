#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{usr_print, usr_get_system_time, usr_get_process_id, usr_thread_exit};

#[global_allocator]
static USER_ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_print("=== System Uptime Demo (Aufgabe 9: Syscalls) ===\n");

    let pid = usr_get_process_id();
    let mut buf = [0u8; 48];
    let len = format_with_num(b"Running as PID: ", pid, &mut buf);
    usr_print(unsafe { core::str::from_utf8_unchecked(&buf[..len]) });

    let ms = usr_get_system_time();
    let total_s = ms / 1000;
    let h = total_s / 3600;
    let m = (total_s % 3600) / 60;
    let s = total_s % 60;

    let mut buf2 = [0u8; 64];
    let prefix = b"System uptime: ";
    let mut pos = prefix.len();
    buf2[..pos].copy_from_slice(prefix);
    pos += write_padded(h, 2, &mut buf2[pos..]);
    buf2[pos] = b':'; pos += 1;
    pos += write_padded(m, 2, &mut buf2[pos..]);
    buf2[pos] = b':'; pos += 1;
    pos += write_padded(s, 2, &mut buf2[pos..]);
    let mid = b" (";
    buf2[pos..pos+mid.len()].copy_from_slice(mid);
    pos += mid.len();
    pos += write_num(ms, &mut buf2[pos..]);
    let suffix = b" ms)\n";
    buf2[pos..pos+suffix.len()].copy_from_slice(suffix);
    pos += suffix.len();
    usr_print(unsafe { core::str::from_utf8_unchecked(&buf2[..pos]) });

    usr_thread_exit();
}

fn format_with_num(prefix: &[u8], val: usize, buf: &mut [u8]) -> usize {
    buf[..prefix.len()].copy_from_slice(prefix);
    let dlen = write_num(val, &mut buf[prefix.len()..]);
    buf[prefix.len() + dlen] = b'\n';
    prefix.len() + dlen + 1
}

fn write_padded(val: usize, width: usize, buf: &mut [u8]) -> usize {
    let mut digits = [0u8; 20];
    let mut n = val;
    let mut len = 0;
    if n == 0 { digits[0] = b'0'; len = 1; }
    else { while n > 0 { digits[len] = b'0' + (n % 10) as u8; n /= 10; len += 1; } }
    let pad = if width > len { width - len } else { 0 };
    for i in 0..pad { buf[i] = b'0'; }
    for i in 0..len { buf[pad + i] = digits[len - 1 - i]; }
    pad + len
}

fn write_num(val: usize, buf: &mut [u8]) -> usize {
    write_padded(val, 0, buf)
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
