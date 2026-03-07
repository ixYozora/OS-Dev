/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: user_threads (Aufgabe 8)                                       ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Test program for Ring 3 user-mode threads. Creates one kernel  ║
   ║         thread and one user thread, each filling a CGA text row with   ║
   ║         'K' or 'U' characters to verify correct privilege levels.      ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Yozora                                                         ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::threads::thread::Thread;

const CGA_BASE: usize = 0xB8000;
const CGA_COLUMNS: usize = 80;

fn write_cga(row: usize, col: usize, ch: u8, attr: u8) {
    let offset = (row * CGA_COLUMNS + col) * 2;
    unsafe {
        let ptr = CGA_BASE as *mut u8;
        ptr.add(offset).write_volatile(ch);
        ptr.add(offset + 1).write_volatile(attr);
    }
}

fn spin_delay() {
    for _ in 0..500_000 {
        core::hint::spin_loop();
    }
}

fn kernel_thread_entry() {
    let mut col: usize = 0;
    loop {
        write_cga(10, col, b'K', 0x0A); // green on black
        col = (col + 1) % CGA_COLUMNS;
        spin_delay();
    }
}

fn user_thread_entry() {
    let mut col: usize = 0;
    loop {
        write_cga(12, col, b'U', 0x0E); // yellow on black
        col = (col + 1) % CGA_COLUMNS;
        spin_delay();
    }
}

pub fn run() {
    let scheduler = get_scheduler();

    let kt = Thread::new_kernel_thread(kernel_thread_entry);
    scheduler.ready(kt);

    let ut = Thread::new_user_thread("hello");
    scheduler.ready(ut);
}
