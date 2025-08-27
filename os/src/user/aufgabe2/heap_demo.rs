use crate::devices::buff_print::{buff_print, buff_set_color};
use crate::devices::lfb::YELLOW;
use crate::kernel::allocator;
use alloc::boxed::Box;
use crate::buff_print;

// A simple struct to allocate on the heap
struct TestStruct {
    x: u32,
    y: u32,
}

pub fn run() {
    buff_set_color(YELLOW);

    buff_print!("Starting simple Heap Demo...\n");

    // --- State Pre-Allocation ---
    buff_print!("\n=== Memory State: Pre-Allocation ===\n");
    allocator::dump_free_list_lfb();

    // --- Allocation ---
    buff_print!("\n=== Allocating a TestStruct ===\n");
    let my_box = Box::new(TestStruct { x: 10, y: 20 });
    buff_print!("Successfully allocated object at address: {:p}\n", my_box);
    buff_print!("Object contents: x={}, y={}\n", my_box.x, my_box.y);

    // --- State Post-Allocation ---
    buff_print!("\n=== Memory State: Post-Allocation ===\n");
    allocator::dump_free_list_lfb();

    // The 'drop' function is used here to explicitly deallocate the memory immediately
    // so we can observe the state change. Without 'drop', the deallocation would happen
    // automatically at the end of the function's scope.
    buff_print!("\n=== Explicitly Deallocating the TestStruct ===\n");
    drop(my_box);
    buff_print!("Object deallocated.\n");

    // --- State Post-Deallocation ---
    buff_print!("\n=== Memory State: Post-Deallocation ===\n");
    allocator::dump_free_list_lfb();

    buff_print!("\nDemo complete.\n");
}