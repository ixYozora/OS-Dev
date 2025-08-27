use crate::devices::cga; // optional
use crate::devices::cga_print;
use crate::devices::cga_print::print;
use crate::{buff_print, devices::buff_print::{buff_print, buff_clear, buff_set_color}};
use crate::devices::lfb::WHITE;
use crate::devices::lfb::{color, get_lfb, HHU_GREEN};
use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::threads::thread::Thread;

pub fn run() {


    buff_print!("Test der Zahlenausgabenfunktion:\n");
    buff_print!("\n");
    buff_print!("dex    hex   bin\n");

    for i in 0..=16 {
        buff_print!("{:3}   0x{:02x}  {:04b}\n", i, i, i);
    }

    buff_print!("Demo finished.\n");

}

