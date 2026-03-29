#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::boxed::Box;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{usr_print, usr_get_key, usr_thread_exit, usr_map_heap};

#[global_allocator]
static USER_ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

const USER_HEAP_START: u64 = 0x200_0000_0000;
const USER_HEAP_SIZE: usize = 512 * 1024;

struct TestStruct {
    x: u32,
    y: u32,
}

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_map_heap(USER_HEAP_START, USER_HEAP_SIZE);
    unsafe {
        USER_ALLOCATOR.lock().init_at(USER_HEAP_START as usize, USER_HEAP_SIZE);
    }

    usr_print("Starting simple Heap Demo (user heap)...\n\n");
    usr_print("=== Allocating a TestStruct ===\n");
    let my_box = Box::new(TestStruct { x: 1, y: 2 });
    usr_print("Successfully allocated object.\n");
    usr_print("Object contents: x=");
    print_u32(my_box.x);
    usr_print(", y=");
    print_u32(my_box.y);
    usr_print("\n");

    usr_print("Press any key to deallocate...\n");
    let _ = usr_get_key();

    usr_print("\n=== Explicitly Deallocating the TestStruct ===\n");
    drop(my_box);
    usr_print("Object deallocated.\n\nDemo complete.\n");

    usr_thread_exit();
}

fn print_u32(v: u32) {
    let mut buf = [0u8; 12];
    let mut n = v;
    let mut i = 0;
    if n == 0 {
        buf[0] = b'0';
        i = 1;
    } else {
        while n > 0 {
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
            i += 1;
        }
    }
    for j in 0..i / 2 {
        buf.swap(j, i - 1 - j);
    }
    if let Ok(s) = core::str::from_utf8(&buf[..i]) {
        usr_print(s);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
