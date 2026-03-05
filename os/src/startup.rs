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
#![feature(naked_functions)]

extern crate alloc;
extern crate spin; // we need a mutex in devices::cga_print

// insert other modules
#[macro_use] // import macros, too
mod devices;
mod kernel;
mod user;
mod consts;

mod library;

use core::panic::PanicInfo;

use devices::cga; // shortcut for cga
use devices::cga_print; // used to import code needed by println!
use devices::keyboard; // shortcut for keyboard
use devices::pit;
use kernel::allocator;
use kernel::cpu;
use user::aufgabe1::text_demo;
use user::aufgabe1::keyboard_demo;
use user::aufgabe2::heap_demo;
use user::aufgabe2::sound_demo;
use user::aufgabe4::coroutine_demo;
use user::aufgabe4::thread_demo;
use user::aufgabe5::aufgabe5_demo;
use crate::user::aufgabe7::graphic_demo;
use crate::user::aufgabe7::yozorashell;
use kernel::interrupts::idt;
use kernel::interrupts::pic;
use kernel::interrupts::intdispatcher;
use kernel::interrupts::intdispatcher::INT_VECTORS;
use crate::cpu::IoPort;
use crate::kernel::multiboot::FramebufferType;
use crate::kernel::multiboot::MultibootInfo;
use crate::devices::pci::get_pci_bus;
use crate::devices::lfb::init_lfb;
use crate::devices::pci::Command;
use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::threads::thread::Thread;

#[unsafe(no_mangle)]
pub extern "C" fn startup(multiboot_info: &MultibootInfo) {
    // Copy multiboot info to stack — the original lies in physical memory
    // that may be reused by the page frame allocator
    let multiboot_info = *multiboot_info;

    kprintln!("Initializing physical memory allocator");
    multiboot_info.init_phys_memory_allocator();

    kprintln!("Testing page frame allocator");
    kernel::paging::frames::test_frame_allocator();

    allocator::init();
    kprintln!("Initializing allocator");

    kprintln!("Initializing kernel page tables");
    let kernel_pml4 = kernel::paging::pages::init_kernel_tables();
    unsafe { kernel::paging::pages::write_cr3(kernel_pml4); }
    kprintln!("Kernel page tables active");

    kprintln!("Initializing PIC");
    pic::PIC.lock().init();

    kprintln!("Initializing interrupts");
    idt::get_idt().load();
    intdispatcher::INT_VECTORS.lock().init();

    kprintln!("Initializing keyboard");
    keyboard::plugin();

    kprintln!("Enabling interrupts");
    cpu::enable_int();

    kprintln!("Initializing PIT");
    pit::plugin();

    kprintln!("Initializing fast syscalls (syscall/sysret)");
    kernel::syscalls::syscall_dispatcher::init_fast_syscalls();

    kprintln!("Boot sequence finished");

    kprintln!("Scanning PCI bus");
    for device in get_pci_bus().iter() {
        kprintln!("Found PCI device {:04x}:{:04x}", device.read_vendor_id(), device.read_device_id());
    }

    if let Some(framebuffer_info) = multiboot_info.get_framebuffer_info() {
        match framebuffer_info.typ {
            FramebufferType::Indexed => {
                panic!("Color palette framebuffer not supported!");
            }
            FramebufferType::RGB => {
                init_lfb(
                    framebuffer_info.addr as *mut u8,
                    framebuffer_info.pitch,
                    framebuffer_info.width,
                    framebuffer_info.height,
                    framebuffer_info.bpp
                );

                // Get scheduler
                let scheduler = get_scheduler();

                // Create shell thread
                let shell_thread = Thread::new_kernel_thread(yozorashell::launch);
                scheduler.ready(shell_thread);

                // Start the scheduler: idle thread + shell thread + any future threads
                scheduler.schedule();
            }
            FramebufferType::Text => {
                cga::CGA.lock().clear();
                user::aufgabe11::paging_demo::paging_test();
            }
        }
    } else {
       

    }
}




#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("Panic: {}", info);
    //	kprintln!("{:?}", Backtrace::new());
    loop {}
}

