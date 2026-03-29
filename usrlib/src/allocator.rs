use alloc::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};
use crate::spinlock::{Spinlock, SpinlockGuard};

/// A wrapper around `Spinlock` to allow for trait implementations (e.g. GlobalAlloc).
pub struct Locked<A> {
    inner: Spinlock<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: Spinlock::new(inner),
        }
    }

    pub fn lock(&self) -> SpinlockGuard<'_, A> {
        self.inner.lock()
    }

    pub fn is_locked(&self) -> bool {
        self.inner.is_locked()
    }
}

pub fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr % align;
    if remainder == 0 {
        addr
    } else {
        addr - remainder + align
    }
}

struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

/// A linked list allocator that uses a free list to manage memory.
pub struct LinkedListAllocator {
    head: ListNode,
    heap_start: usize,
    heap_end: usize,
}

impl LinkedListAllocator {
    pub const fn new(heap_start: usize, heap_size: usize) -> LinkedListAllocator {
        LinkedListAllocator {
            head: ListNode::new(heap_size),
            heap_start,
            heap_end: heap_start + heap_size,
        }
    }

    pub unsafe fn init(&mut self) {
        let start = self.heap_start;
        let size = self.heap_end - self.heap_start;
        self.head.next = None;
        unsafe {
            self.add_free_block(start, size);
        }
    }

    pub unsafe fn init_at(&mut self, start: usize, size: usize) {
        self.heap_start = start;
        self.heap_end = start + size;
        self.head.next = None;
        unsafe {
            self.add_free_block(start, size);
        }
    }

    unsafe fn add_free_block(&mut self, addr: usize, size: usize) {
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        unsafe {
            node_ptr.write(node);
            self.head.next = Some(&mut *node_ptr)
        }
    }

    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(size_of::<ListNode>());
        (size, layout.align())
    }

    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let (size, align) = Self::size_align(layout);
        let mut prev = &mut self.head;

        while let Some(ref mut node) = prev.next {
            let alloc_start = align_up(node.start_addr() + mem::size_of::<ListNode>(), align);
            let alloc_end = alloc_start.checked_add(size).unwrap();

            if alloc_end <= node.end_addr() {
                let excess_size = node.end_addr() - alloc_end;
                let split = excess_size >= mem::size_of::<ListNode>();

                let node_next = node.next.take();
                prev.next = node_next;

                if split {
                    unsafe {
                        self.add_free_block(alloc_end, excess_size);
                    }
                }

                return alloc_start as *mut u8;
            }
            prev = prev.next.as_mut().unwrap();
        }
        ptr::null_mut()
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let (size, _) = Self::size_align(layout);
        let block_start = ptr as usize - mem::size_of::<ListNode>();
        let total_size = size + mem::size_of::<ListNode>();

        unsafe {
            self.add_free_block(block_start, total_size);
        }
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            self.lock().alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.lock().dealloc(ptr, layout);
        }
    }
}
