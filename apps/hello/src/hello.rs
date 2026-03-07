#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::boxed::Box;
use alloc::vec::Vec;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{usr_hello_world, usr_get_process_id, usr_print, usr_thread_exit, usr_dump_vmas, usr_map_heap};

#[global_allocator]
static USER_ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

const USER_HEAP_START: u64 = 0x200_0000_0000;
const USER_HEAP_SIZE: usize = 1024 * 1024; // 1 MiB

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_hello_world();

    let pid = usr_get_process_id();
    let mut buf = [0u8; 32];
    let msg = format_pid(pid, &mut buf);
    usr_print(msg);

    usr_map_heap(USER_HEAP_START, USER_HEAP_SIZE);
    unsafe {
        USER_ALLOCATOR.lock().init_at(USER_HEAP_START as usize, USER_HEAP_SIZE);
    }

    let val = Box::new(42u64);
    let mut buf2 = [0u8; 32];
    let msg2 = format_box_val(*val, &mut buf2);
    usr_print(msg2);

    let mut v: Vec<u32> = Vec::with_capacity(10);
    for i in 0..10 {
        v.push(i);
    }
    let mut buf3 = [0u8; 32];
    let msg3 = format_vec_len(v.len(), &mut buf3);
    usr_print(msg3);

    usr_dump_vmas();

    let result = deep_recursion(20, [0u64; 64]);
    let mut buf4 = [0u8; 32];
    let msg4 = format_num_with_prefix(b"Fib(20): ", result as usize, &mut buf4);
    usr_print(msg4);

    usr_thread_exit();
}

fn format_pid(pid: usize, buf: &mut [u8]) -> &str {
    let prefix = b"Process ID: ";
    format_num_with_prefix(prefix, pid, buf)
}

fn format_box_val(val: u64, buf: &mut [u8]) -> &str {
    let prefix = b"Box value: ";
    format_num_with_prefix(prefix, val as usize, buf)
}

fn format_vec_len(len: usize, buf: &mut [u8]) -> &str {
    let prefix = b"Vec len: ";
    format_num_with_prefix(prefix, len, buf)
}

fn format_num_with_prefix<'a>(prefix: &[u8], num: usize, buf: &'a mut [u8]) -> &'a str {
    let prefix_len = prefix.len();
    buf[..prefix_len].copy_from_slice(prefix);

    if num == 0 {
        buf[prefix_len] = b'0';
        buf[prefix_len + 1] = b'\n';
        unsafe { core::str::from_utf8_unchecked(&buf[..prefix_len + 2]) }
    } else {
        let mut n = num;
        let mut digits = [0u8; 20];
        let mut len = 0;
        while n > 0 {
            digits[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
        for i in 0..len {
            buf[prefix_len + i] = digits[len - 1 - i];
        }
        buf[prefix_len + len] = b'\n';
        unsafe { core::str::from_utf8_unchecked(&buf[..prefix_len + len + 1]) }
    }
}

/// Recursive function with large stack frames to test dynamic stack growth.
/// Each call uses ~512 bytes of stack (64 u64s = 512 bytes for the padding array).
#[inline(never)]
fn deep_recursion(n: u64, _padding: [u64; 64]) -> u64 {
    if n <= 1 {
        return n;
    }
    let a = deep_recursion(n - 1, [n; 64]);
    let b = deep_recursion(n - 2, [n; 64]);
    a + b
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
