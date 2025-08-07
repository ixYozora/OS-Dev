use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::kernel::allocator;


struct TestStruct {
    x: u32,
    y: u32,
    data: [u8; 100],
}

pub fn run() {
    kprintln!("Starting heap demo...");
     allocator::dump_free_list();
    //
    // // Test 1: Allocate a single struct
    // kprintln!("Test 1: Allocating a single struct");
    // let box1 = Box::new(TestStruct {
    //     x: 42,
    //     y: 100,
    //     data: [0; 100],
    // });
    //
    // allocator::dump_free_list();
    // drop(box1); // Deallocate memory
    // kprintln!("");
    // Test 2: Allocate and deallocate memory
    kprintln!("Test 2: Allocating and deallocating memory");
    let box2 = Box::new(TestStruct {
        x: 1,
        y: 2,
        data: [0; 100],
    });
    allocator::dump_free_list();
    drop(box2); // Deallocate memory
    allocator::dump_free_list();



}
