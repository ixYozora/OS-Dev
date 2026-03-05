use core::fmt;
use core::ops::{Add, Sub};
use crate::consts::PAGE_FRAME_SIZE;
use crate::library::spinlock::Spinlock as Mutex;

pub static FRAME_ALLOCATOR: Mutex<PfListAllocator> = Mutex::new(PfListAllocator::new());

/// Represents a physical address in memory and allows accessing it via pointers.
#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct PhysAddr(u64);

impl PhysAddr {
    pub const fn new(addr: u64) -> Self {
        PhysAddr(addr)
    }

    pub fn raw(&self) -> u64 {
        self.0
    }

    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    pub fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Phys(0x{:016x})", self.0)
    }
}

impl From<PhysAddr> for u64 {
    fn from(addr: PhysAddr) -> Self {
        addr.0
    }
}

impl Add<PhysAddr> for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: PhysAddr) -> Self::Output {
        PhysAddr(self.0.checked_add(rhs.0).unwrap())
    }
}

impl Sub<PhysAddr> for PhysAddr {
    type Output = PhysAddr;
    fn sub(self, rhs: PhysAddr) -> Self::Output {
        PhysAddr(self.0.checked_sub(rhs.0).unwrap())
    }
}

impl Add<usize> for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: usize) -> Self::Output {
        PhysAddr(self.0.checked_add(rhs as u64).unwrap())
    }
}

impl Sub<usize> for PhysAddr {
    type Output = PhysAddr;
    fn sub(self, rhs: usize) -> Self::Output {
        PhysAddr(self.0.checked_sub(rhs as u64).unwrap())
    }
}

impl Add<u64> for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: u64) -> Self::Output {
        PhysAddr(self.0.checked_add(rhs).unwrap())
    }
}

impl Sub<u64> for PhysAddr {
    type Output = PhysAddr;
    fn sub(self, rhs: u64) -> Self::Output {
        PhysAddr(self.0.checked_sub(rhs).unwrap())
    }
}

/// A node in the physical frame free list.
/// Stored directly inside the free memory block it describes.
struct PfListNode {
    size: usize,
    next: Option<&'static mut PfListNode>
}

impl PfListNode {
    const fn new(size: usize) -> Self {
        PfListNode { size, next: None }
    }

    fn start_addr(&self) -> PhysAddr {
        PhysAddr::new(self as *const Self as u64)
    }

    fn end_addr(&self) -> PhysAddr {
        self.start_addr() + self.size
    }
}

/// Physical frame allocator using a sorted free-list with coalescing.
/// Free blocks are tracked as linked-list nodes written directly into the free memory.
/// Blocks are always multiples of PAGE_FRAME_SIZE (4 KiB).
pub struct PfListAllocator {
    head: PfListNode
}

impl PfListAllocator {
    pub const fn new() -> PfListAllocator {
        PfListAllocator {
            head: PfListNode::new(0)
        }
    }

    /// Allocate a contiguous block of `num_frames` page frames.
    /// Returns the starting physical address on success.
    /// The allocated memory is zero-filled.
    pub unsafe fn alloc_block(&mut self, num_frames: usize) -> Option<PhysAddr> {
        assert!(num_frames > 0, "Cannot allocate 0 frames");
        let needed = num_frames * PAGE_FRAME_SIZE;

        let mut prev = &mut self.head;
        while let Some(ref mut node) = prev.next {
            if node.size >= needed {
                let alloc_addr = node.start_addr();
                let excess = node.size - needed;

                if excess > 0 {
                    // Split: write remainder node after the allocated region
                    let rem_addr = alloc_addr + needed;
                    let after = node.next.take();
                    let rem_ptr = rem_addr.as_mut_ptr::<PfListNode>();
                    unsafe {
                        rem_ptr.write(PfListNode { size: excess, next: after });
                        prev.next = Some(&mut *rem_ptr);
                    }
                } else {
                    // Exact fit: remove node from list
                    let after = node.next.take();
                    prev.next = after;
                }

                // Zero-fill the allocated block
                unsafe {
                    core::ptr::write_bytes(alloc_addr.as_mut_ptr::<u8>(), 0, needed);
                }

                return Some(alloc_addr);
            }
            prev = prev.next.as_mut().unwrap();
        }

        None
    }

    /// Free a block of `num_frames` page frames at `addr`.
    /// The block is inserted into the sorted free-list and merged with adjacent blocks.
    pub unsafe fn free_block(&mut self, addr: PhysAddr, num_frames: usize) {
        assert!(num_frames > 0, "Cannot free 0 frames");
        assert!(
            addr.raw() % PAGE_FRAME_SIZE as u64 == 0,
            "Address {:#x} not page-aligned", addr.raw()
        );

        let block_size = num_frames * PAGE_FRAME_SIZE;

        // Find insertion point: walk until prev.next.start >= addr
        let mut prev = &mut self.head;
        while let Some(ref next_node) = prev.next {
            if next_node.start_addr() >= addr {
                break;
            }
            prev = prev.next.as_mut().unwrap();
        }

        // Check if we can coalesce with the previous block.
        // The head sentinel has size 0 and lives in static memory, so end_addr != any
        // valid physical address — it naturally won't match.
        let merge_prev = prev.size > 0 && prev.end_addr() == addr;

        // Check if we can coalesce with the next block
        let merge_next = prev.next.as_ref()
            .map_or(false, |n| PhysAddr::new(addr.raw() + block_size as u64) == n.start_addr());

        match (merge_prev, merge_next) {
            (true, true) => {
                // Merge with both neighbors
                let next = prev.next.as_mut().unwrap();
                prev.size += block_size + next.size;
                let after = next.next.take();
                prev.next = after;
            }
            (true, false) => {
                // Extend previous block
                prev.size += block_size;
            }
            (false, true) => {
                // Merge with next: create new node at addr spanning both
                let next = prev.next.as_mut().unwrap();
                let new_size = block_size + next.size;
                let after = next.next.take();
                prev.next.take(); // detach old next
                unsafe {
                    let node_ptr = addr.as_mut_ptr::<PfListNode>();
                    node_ptr.write(PfListNode { size: new_size, next: after });
                    prev.next = Some(&mut *node_ptr);
                }
            }
            (false, false) => {
                // No merge: insert new standalone node
                let after = prev.next.take();
                unsafe {
                    let node_ptr = addr.as_mut_ptr::<PfListNode>();
                    node_ptr.write(PfListNode { size: block_size, next: after });
                    prev.next = Some(&mut *node_ptr);
                }
            }
        }
    }

    /// Print the free-list to the serial port.
    pub fn dump_free_list(&self) {
        kprintln!("Physical frame allocator free list:");
        let mut current = &self.head;
        let mut total_frames: usize = 0;
        while let Some(ref node) = current.next {
            let frames = node.size / PAGE_FRAME_SIZE;
            kprintln!("  {:?} - {:?}  ({} frames, {} KB)",
                node.start_addr(), node.end_addr(), frames, node.size / 1024);
            total_frames += frames;
            current = current.next.as_ref().unwrap();
        }
        kprintln!("  Total free: {} frames ({} MB)",
            total_frames, total_frames * PAGE_FRAME_SIZE / (1024 * 1024));
    }
}

/// Test the physical frame allocator for correctness.
/// Allocates and frees blocks, checks coalescing and non-overlap.
pub fn test_frame_allocator() {
    kprintln!("--- Page Frame Allocator Test ---");

    let mut alloc = FRAME_ALLOCATOR.lock();
    alloc.dump_free_list();

    // Allocate three single-frame blocks
    let a = unsafe { alloc.alloc_block(1).expect("alloc a failed") };
    let b = unsafe { alloc.alloc_block(1).expect("alloc b failed") };
    let c = unsafe { alloc.alloc_block(1).expect("alloc c failed") };
    kprintln!("Allocated: a={:?}, b={:?}, c={:?}", a, b, c);

    // Verify non-overlap
    assert!(a != b && b != c && a != c, "Allocated blocks overlap!");

    // Verify zero-filled
    unsafe {
        let first_byte = *(a.as_ptr::<u8>());
        assert!(first_byte == 0, "Allocated block not zero-filled!");
    }

    // Verify page-alignment
    assert!(a.raw() % PAGE_FRAME_SIZE as u64 == 0);
    assert!(b.raw() % PAGE_FRAME_SIZE as u64 == 0);
    assert!(c.raw() % PAGE_FRAME_SIZE as u64 == 0);

    // Free in reverse order → should coalesce back
    unsafe { alloc.free_block(c, 1); }
    unsafe { alloc.free_block(b, 1); }
    unsafe { alloc.free_block(a, 1); }
    kprintln!("Freed a, b, c — list after coalescing:");
    alloc.dump_free_list();

    // Allocate a large block
    let big = unsafe { alloc.alloc_block(256).expect("alloc 256 frames failed") };
    kprintln!("Allocated 256 frames (1 MB) at {:?}", big);
    unsafe { alloc.free_block(big, 256); }
    kprintln!("Freed 256 frames — list after free:");
    alloc.dump_free_list();

    kprintln!("--- All page frame allocator tests passed ---");
}
