use alloc::alloc::Layout;
use usrlib::allocator::{Locked, LinkedListAllocator};
use crate::consts;
use crate::consts::PAGE_FRAME_SIZE;
use crate::kernel::paging::frames::FRAME_ALLOCATOR;

const HEAP_SIZE: usize = consts::HEAP_SIZE;

#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

/// Allocate heap memory from the page frame allocator and initialize the heap.
pub fn init() {
    let heap_frames = HEAP_SIZE / PAGE_FRAME_SIZE;
    let heap_start = unsafe {
        FRAME_ALLOCATOR.lock()
            .alloc_block(heap_frames)
            .expect("Failed to allocate heap from page frame allocator")
    };

    let start = heap_start.raw() as usize;
    kprintln!("Heap: {:#x} - {:#x} ({} frames, {} MB)",
        start, start + HEAP_SIZE, heap_frames, HEAP_SIZE / (1024 * 1024));

    unsafe {
        ALLOCATOR.lock().init_at(start, HEAP_SIZE);
    }
}

pub fn alloc(layout: Layout) -> *mut u8 {
    unsafe {
        ALLOCATOR.lock().alloc(layout)
    }
}

pub fn dealloc(ptr: *mut u8, layout: Layout) {
    unsafe {
        ALLOCATOR.lock().dealloc(ptr, layout)
    }
}

pub fn is_locked() -> bool {
    ALLOCATOR.is_locked()
}
