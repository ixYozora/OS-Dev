/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: startup                                                         ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Here is the main function called first from the boot code as    ║
   ║         well as the panic handler. All features are set and all modules ║
   ║         are imported.                                                   ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Michael Schoettner, Univ. Duesseldorf, 5.2.2024                 ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
#![no_std]
#![allow(dead_code)] // avoid warnings
#![allow(unused_variables)] // avoid warnings
#![allow(unused_imports)]
#![allow(unused_macros)]
#![feature(abi_x86_interrupt)]

extern crate alloc;
extern crate spin; // we need a mutex in devices::cga_print

// insert other modules
#[macro_use] // import macros, too
mod devices;
mod kernel;
mod user;
mod consts;

use core::panic::PanicInfo;

use devices::cga; // shortcut for cga
use devices::cga_print; // used to import code needed by println! 
use devices::keyboard; // shortcut for keyboard

use kernel::allocator;
use kernel::cpu;
use user::aufgabe1::text_demo;
use user::aufgabe1::keyboard_demo;
use user::aufgabe2::heap_demo;
use user::aufgabe2::sound_demo;
use kernel::interrupts::idt;
use kernel::interrupts::pic;
use kernel::interrupts::intdispatcher;
use kernel::interrupts::intdispatcher::INT_VECTORS;

fn aufgabe1() {
    text_demo::run();
    kprintln!("Welcome to hhuTOS!");

    println!("Test der Zahlenausgabenfunktion:");
    println!("");
    println!("dex    hex   bin");
    for i in 0..=16 {
        println!("{:3}   0x{:02x}  {:04b}", i, i, i);
    }
    // println!("");
    keyboard_demo::run();
}
fn aufgabe2() {
    heap_demo::run();
    sound_demo::run();
}

fn aufgabe3(){
    kprintln!("Initializing PIC");
    pic::PIC.lock().init();
    
    kprintln!("Initializing interrupts");
    INT_VECTORS.lock().init();
    idt::get_idt().load();
    
    kprintln!("Initializing keyboard");
    keyboard::plugin();
    
    kprintln!("Enabling interrupts");
    cpu::enable_int();
}

#[unsafe(no_mangle)]
pub extern "C" fn startup() {
    kprintln!("Welcome to hhuTOS!");
    kprintln!("Initializing heap allocator");
    allocator::init();
    cga::CGA.lock().clear();
    
    // Initialize interrupts and keyboard
    aufgabe3();
    
    kprintln!("Boot sequence finished");
    
    // Keep the OS running to handle interrupts
    loop {
        // Process keyboard input from the buffer
        if let Some(key) = keyboard::get_key_buffer().get_last_key() {
            if key.valid() {
                let asc = key.get_ascii();
                if asc == 8 { // Backspace
                    let mut cga = cga::CGA.lock();
                    let (mut x, mut y) = cga.getpos();
                    if x > 0 || y > 0 {
                        if x == 0 {
                            x = 79;
                            if y > 0 { y -= 1; }
                        } else {
                            x -= 1;
                        }
                        cga.setpos(x, y);
                        cga.show(x, y, ' ', cga::CGA_STD_ATTR);
                        cga.setpos(x, y);
                    }
                }
                else if asc != 0 && (asc.is_ascii_graphic() || asc == b' ') {
                    print!("{}", asc as char);
                }
            }
        }
    }
}



#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("Panic: {}", info);
    //	kprintln!("{:?}", Backtrace::new());
    loop {}
}

