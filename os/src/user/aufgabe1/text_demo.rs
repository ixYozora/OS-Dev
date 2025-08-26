use crate::devices::cga; // optional
use crate::devices::cga_print;
use crate::devices::cga_print::print;
use crate::lfb_print;
use crate::{lfb_println, devices::lfb_print::{lfb_print, lfb_clear, lfb_set_color}};
use crate::devices::lfb::WHITE;
use crate::devices::lfb::{color, get_lfb, HHU_GREEN};
use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::threads::thread::Thread;

pub fn run() {


    lfb_println!("Test der Zahlenausgabenfunktion:");
    lfb_println!("");
    lfb_println!("dex    hex   bin");

    for i in 0..=16 {
        lfb_println!("{:3}   0x{:02x}  {:04b}", i, i, i);
    }

    lfb_println!("Demo finished.");

}

