use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::kernel::allocator;
use crate::devices::lfb_print::{lfb_print, lfb_set_color};
use crate::devices::lfb::{HHU_GREEN, WHITE};
use crate::lfb_print;

struct TestStruct {
    x: u32,
    y: u32,
    data: [u8; 100],
}

pub fn run() {
    lfb_print!("=== Heap Demo ===\n");

    // Show only key states to fit in console
    lfb_print!("Start: ");
    allocator::dump_free_list_lfb();

    // Allocate and immediately show fragmentation
    let box1 = Box::new(TestStruct { x: 1, y: 2, data: [1; 100] });
    let box2 = Box::new(TestStruct { x: 3, y: 4, data: [2; 100] });
    drop(box1); // Create hole in middle

    lfb_print!("Fragmented: ");
    allocator::dump_free_list_lfb();

    drop(box2);
    lfb_print!("End: ");
    allocator::dump_free_list_lfb();

    lfb_set_color(HHU_GREEN);
    lfb_print!("yozora$ ");
    lfb_set_color(WHITE);
}