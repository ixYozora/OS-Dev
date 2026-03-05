/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: allocator                                                       ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Implementing functions for the heap allocator used by the rust  ║
   ║         compiler. Heap memory is obtained from the page frame allocator ║
   ║         at runtime.                                                     ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Philipp Oppermann                                               ║
   ║         https://os.phil-opp.com/allocator-designs/                      ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use alloc::alloc::Layout;
use crate::kernel::allocator::list::LinkedListAllocator;
use crate::consts;
use crate::consts::PAGE_FRAME_SIZE;
use crate::kernel::paging::frames::FRAME_ALLOCATOR;

pub mod bump;
pub mod list;

const HEAP_SIZE: usize = consts::HEAP_SIZE;

// Allocator is created with dummy values; init() sets the real heap range.
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

pub fn dump_free_list_lfb() {
    ALLOCATOR.lock().dump_free_list();
}

/// A wrapper around `spin::Mutex` to allow for trait implementations.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

/// Helper function used in `bump.rs` and `list.rs`.
fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr % align;
    if remainder == 0 {
        addr
    } else {
        addr - remainder + align
    }
}

pub fn is_locked() -> bool {
    ALLOCATOR.inner.is_locked()
}
