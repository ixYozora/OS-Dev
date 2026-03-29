#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{usr_pcspk_play, usr_thread_exit};

#[global_allocator]
static _A: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_pcspk_play(0);
    usr_thread_exit();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
