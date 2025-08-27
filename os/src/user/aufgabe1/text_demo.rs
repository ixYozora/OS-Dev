use crate::devices::cga; // optional
use crate::devices::cga_print;
use crate::devices::cga_print::print;
use crate::{lfb_print, devices::buff_print::{lfb_print, lfb_clear, lfb_set_color}};
use crate::devices::lfb::WHITE;
use crate::devices::lfb::{color, get_lfb, HHU_GREEN};
use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::threads::thread::Thread;

pub fn run() {


    lfb_print!("Test der Zahlenausgabenfunktion:\n");
    lfb_print!("\n");
    lfb_print!("dex    hex   bin\n");

    for i in 0..=16 {
        lfb_print!("{:3}   0x{:02x}  {:04b}\n", i, i, i);
    }

    lfb_print!("Demo finished.\n");

}

