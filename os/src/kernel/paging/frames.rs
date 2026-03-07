use core::fmt;
use core::ops::{Add, Sub};
use crate::consts::PAGE_FRAME_SIZE;
use usrlib::spinlock::Spinlock as Mutex;

pub static FRAME_ALLOCATOR: Mutex<PfListAllocator> = Mutex::new(PfListAllocator::new());

/// Represents a physical address in memory.
#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct PhysAddr(u64);

impl PhysAddr {
    pub const fn new(addr: u64) -> Self { PhysAddr(addr) }
    pub fn raw(&self) -> u64 { self.0 }
    pub fn as_ptr<T>(&self) -> *const T { self.0 as *const T }
    pub fn as_mut_ptr<T>(&self) -> *mut T { self.0 as *mut T }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Phys(0x{:016x})", self.0)
    }
}

impl From<PhysAddr> for u64 { fn from(a: PhysAddr) -> Self { a.0 } }

impl Add<PhysAddr> for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: PhysAddr) -> Self::Output { PhysAddr(self.0.checked_add(rhs.0).unwrap()) }
}
impl Sub<PhysAddr> for PhysAddr {
    type Output = PhysAddr;
    fn sub(self, rhs: PhysAddr) -> Self::Output { PhysAddr(self.0.checked_sub(rhs.0).unwrap()) }
}
impl Add<usize> for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: usize) -> Self::Output { PhysAddr(self.0.checked_add(rhs as u64).unwrap()) }
}
impl Sub<usize> for PhysAddr {
    type Output = PhysAddr;
    fn sub(self, rhs: usize) -> Self::Output { PhysAddr(self.0.checked_sub(rhs as u64).unwrap()) }
}
impl Add<u64> for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: u64) -> Self::Output { PhysAddr(self.0.checked_add(rhs).unwrap()) }
}
impl Sub<u64> for PhysAddr {
    type Output = PhysAddr;
    fn sub(self, rhs: u64) -> Self::Output { PhysAddr(self.0.checked_sub(rhs).unwrap()) }
}

struct PfListNode {
    size: usize,
    next: Option<&'static mut PfListNode>
}

impl PfListNode {
    const fn new(size: usize) -> Self { PfListNode { size, next: None } }
    fn start_addr(&self) -> PhysAddr { PhysAddr::new(self as *const Self as u64) }
    fn end_addr(&self) -> PhysAddr { self.start_addr() + self.size }
}

/// Physical frame allocator — sorted free-list with coalescing.
pub struct PfListAllocator {
    head: PfListNode,
    max_addr: PhysAddr,
}

impl PfListAllocator {
    pub const fn new() -> PfListAllocator {
        PfListAllocator {
            head: PfListNode::new(0),
            max_addr: PhysAddr::new(0),
        }
    }

    pub fn get_max_phys_addr(&self) -> PhysAddr {
        self.max_addr
    }

    /// Allocate `num_frames` contiguous page frames (zero-filled).
    pub unsafe fn alloc_block(&mut self, num_frames: usize) -> Option<PhysAddr> {
        assert!(num_frames > 0, "Cannot allocate 0 frames");
        let needed = num_frames * PAGE_FRAME_SIZE;

        let mut prev = &mut self.head;
        while let Some(ref mut node) = prev.next {
            if node.size >= needed {
                let alloc_addr = node.start_addr();
                let excess = node.size - needed;

                if excess > 0 {
                    let rem_addr = alloc_addr + needed;
                    let after = node.next.take();
                    let rem_ptr = rem_addr.as_mut_ptr::<PfListNode>();
                    unsafe {
                        rem_ptr.write(PfListNode { size: excess, next: after });
                        prev.next = Some(&mut *rem_ptr);
                    }
                } else {
                    let after = node.next.take();
                    prev.next = after;
                }

                unsafe {
                    core::ptr::write_bytes(alloc_addr.as_mut_ptr::<u8>(), 0, needed);
                }
                return Some(alloc_addr);
            }
            prev = prev.next.as_mut().unwrap();
        }
        None
    }

    /// Free `num_frames` page frames at `addr`. Sorted insert + coalesce.
    pub unsafe fn free_block(&mut self, addr: PhysAddr, num_frames: usize) {
        assert!(num_frames > 0, "Cannot free 0 frames");
        assert!(addr.raw() % PAGE_FRAME_SIZE as u64 == 0,
            "Address {:#x} not page-aligned", addr.raw());

        let block_size = num_frames * PAGE_FRAME_SIZE;

        // Track highest physical address ever seen
        let block_end = PhysAddr::new(addr.raw() + block_size as u64);
        if block_end > self.max_addr {
            self.max_addr = block_end;
        }

        let mut prev = &mut self.head;
        while let Some(ref next_node) = prev.next {
            if next_node.start_addr() >= addr { break; }
            prev = prev.next.as_mut().unwrap();
        }

        let merge_prev = prev.size > 0 && prev.end_addr() == addr;
        let merge_next = prev.next.as_ref()
            .map_or(false, |n| block_end == n.start_addr());

        match (merge_prev, merge_next) {
            (true, true) => {
                let next = prev.next.as_mut().unwrap();
                prev.size += block_size + next.size;
                let after = next.next.take();
                prev.next = after;
            }
            (true, false) => {
                prev.size += block_size;
            }
            (false, true) => {
                let next = prev.next.as_mut().unwrap();
                let new_size = block_size + next.size;
                let after = next.next.take();
                prev.next.take();
                unsafe {
                    let node_ptr = addr.as_mut_ptr::<PfListNode>();
                    node_ptr.write(PfListNode { size: new_size, next: after });
                    prev.next = Some(&mut *node_ptr);
                }
            }
            (false, false) => {
                let after = prev.next.take();
                unsafe {
                    let node_ptr = addr.as_mut_ptr::<PfListNode>();
                    node_ptr.write(PfListNode { size: block_size, next: after });
                    prev.next = Some(&mut *node_ptr);
                }
            }
        }
    }

    pub fn dump_free_list(&self) {
        kprintln!("Physical frame allocator free list (max_addr={:?}):", self.max_addr);
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

pub fn test_frame_allocator() {
    kprintln!("--- Page Frame Allocator Test ---");
    let mut alloc = FRAME_ALLOCATOR.lock();
    alloc.dump_free_list();

    let a = unsafe { alloc.alloc_block(1).expect("alloc a") };
    let b = unsafe { alloc.alloc_block(1).expect("alloc b") };
    let c = unsafe { alloc.alloc_block(1).expect("alloc c") };
    assert!(a != b && b != c && a != c, "Overlap!");
    unsafe { assert!(*(a.as_ptr::<u8>()) == 0, "Not zeroed!"); }

    unsafe { alloc.free_block(c, 1); }
    unsafe { alloc.free_block(b, 1); }
    unsafe { alloc.free_block(a, 1); }

    let big = unsafe { alloc.alloc_block(256).expect("alloc 256") };
    unsafe { alloc.free_block(big, 256); }
    kprintln!("--- All page frame allocator tests passed ---");
}
