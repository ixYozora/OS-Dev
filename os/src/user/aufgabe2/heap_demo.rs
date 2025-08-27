use crate::devices::buff_print::{lfb_print, lfb_set_color};
use crate::devices::lfb::YELLOW;
use crate::kernel::allocator;
use alloc::boxed::Box;
use crate::lfb_print;

// A simple struct to allocate on the heap
struct TestStruct {
    x: u32,
    y: u32,
}

pub fn run() {
    lfb_set_color(YELLOW);

    lfb_print!("Starting simple Heap Demo...\n");

    // --- State Pre-Allocation ---
    lfb_print!("\n=== Memory State: Pre-Allocation ===\n");
    allocator::dump_free_list_lfb();

    // --- Allocation ---
    lfb_print!("\n=== Allocating a TestStruct ===\n");
    let my_box = Box::new(TestStruct { x: 10, y: 20 });
    lfb_print!("Successfully allocated object at address: {:p}\n", my_box);
    lfb_print!("Object contents: x={}, y={}\n", my_box.x, my_box.y);

    // --- State Post-Allocation ---
    lfb_print!("\n=== Memory State: Post-Allocation ===\n");
    allocator::dump_free_list_lfb();

    // The 'drop' function is used here to explicitly deallocate the memory immediately
    // so we can observe the state change. Without 'drop', the deallocation would happen
    // automatically at the end of the function's scope.
    lfb_print!("\n=== Explicitly Deallocating the TestStruct ===\n");
    drop(my_box);
    lfb_print!("Object deallocated.\n");

    // --- State Post-Deallocation ---
    lfb_print!("\n=== Memory State: Post-Deallocation ===\n");
    allocator::dump_free_list_lfb();

    lfb_print!("\nDemo complete.\n");
}