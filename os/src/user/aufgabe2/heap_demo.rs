use crate::devices::buff_print::{buff_print, buff_set_color};
use crate::devices::lfb::YELLOW;
use crate::kernel::allocator;
use alloc::boxed::Box;
use crate::buff_print;
use crate::devices::lfb::get_lfb;
use crate::devices::{keyboard};
use crate::devices::keyboard::get_key_buffer;
struct TestStruct {
    x: u32,
    y: u32,
}

pub fn run() {
    let lfb = get_lfb();
    buff_set_color(YELLOW);

    buff_print!("Starting simple Heap Demo...\n");

    // --- State Pre-Allocation ---
    buff_print!("\n=== Memory State: Pre-Allocation ===\n");
    allocator::dump_free_list_lfb();

    // --- Allocation ---
    buff_print!("\n=== Allocating a TestStruct ===\n");
    let my_box = Box::new(TestStruct { x: 1, y: 2 });
    buff_print!("Successfully allocated object at address: {:p}\n", my_box);
    buff_print!("Object contents: x={}, y={}\n", my_box.x, my_box.y);

    // --- State Post-Allocation ---
    buff_print!("\n=== Memory State: Post-Allocation ===\n");
    allocator::dump_free_list_lfb();

    get_key_buffer().wait_for_key();
    // The 'drop' function is used here to explicitly deallocate the memory immediately
    // so we can observe the state change.
    buff_print!("\n=== Explicitly Deallocating the TestStruct ===\n");
    drop(my_box);
    buff_print!("Object deallocated.\n");

    // --- State Post-Deallocation ---
    buff_print!("\n=== Memory State: Post-Deallocation ===\n");
    allocator::dump_free_list_lfb();



    buff_print!("\nDemo complete.\n");
}