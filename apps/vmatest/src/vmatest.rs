#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::boxed::Box;
use alloc::vec::Vec;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{
    usr_print, usr_get_process_id, usr_thread_exit,
    usr_map_heap, usr_dump_vmas,
};

#[global_allocator]
static USER_ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

const USER_HEAP_START: u64 = 0x200_0000_0000;
const USER_HEAP_SIZE: usize = 512 * 1024; // 512 KiB

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_print("=== VMA & Heap Demo (Aufgabe 13: VMAs, User Heap) ===\n");

    let pid = usr_get_process_id();
    let mut buf = [0u8; 32];
    let len = format_with_num(b"PID: ", pid, &mut buf);
    usr_print(unsafe { core::str::from_utf8_unchecked(&buf[..len]) });

    usr_print("\n--- Before heap allocation ---\n");
    usr_dump_vmas();

    usr_print("\nMapping user heap...\n");
    usr_map_heap(USER_HEAP_START, USER_HEAP_SIZE);
    unsafe {
        USER_ALLOCATOR.lock().init_at(USER_HEAP_START as usize, USER_HEAP_SIZE);
    }

    usr_print("\n--- After heap allocation ---\n");
    usr_dump_vmas();

    usr_print("\nTesting Box::new(1234)...\n");
    let val = Box::new(1234u64);
    let mut buf2 = [0u8; 32];
    let len2 = format_with_num(b"  Box value: ", *val as usize, &mut buf2);
    usr_print(unsafe { core::str::from_utf8_unchecked(&buf2[..len2]) });

    usr_print("Testing Vec with 20 elements...\n");
    let mut v: Vec<u32> = Vec::with_capacity(20);
    for i in 0..20 {
        v.push(i * i);
    }
    let mut buf3 = [0u8; 32];
    let len3 = format_with_num(b"  Vec len: ", v.len(), &mut buf3);
    usr_print(unsafe { core::str::from_utf8_unchecked(&buf3[..len3]) });

    let mut buf4 = [0u8; 32];
    let len4 = format_with_num(b"  Vec[19]: ", v[19] as usize, &mut buf4);
    usr_print(unsafe { core::str::from_utf8_unchecked(&buf4[..len4]) });

    usr_print("\nDynamic stack growth test (deep recursion)...\n");
    let fib = deep_recursion(15, [0u64; 64]);
    let mut buf5 = [0u8; 32];
    let len5 = format_with_num(b"  Fib(15): ", fib as usize, &mut buf5);
    usr_print(unsafe { core::str::from_utf8_unchecked(&buf5[..len5]) });

    usr_print("\nAll tests passed.\n");
    usr_thread_exit();
}

fn format_with_num(prefix: &[u8], val: usize, buf: &mut [u8]) -> usize {
    buf[..prefix.len()].copy_from_slice(prefix);
    let dlen = write_num(val, &mut buf[prefix.len()..]);
    buf[prefix.len() + dlen] = b'\n';
    prefix.len() + dlen + 1
}

fn write_num(val: usize, buf: &mut [u8]) -> usize {
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut digits = [0u8; 20];
    let mut n = val;
    let mut len = 0;
    while n > 0 {
        digits[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    for i in 0..len {
        buf[i] = digits[len - 1 - i];
    }
    len
}

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
