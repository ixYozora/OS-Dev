use core::ptr;
use crate::consts::{PAGE_SIZE, STACK_SIZE, USER_CODE_VIRT_START, USER_STACK_VIRT_END, USER_STACK_VIRT_START};
use crate::kernel::paging::frames::{PhysAddr, FRAME_ALLOCATOR};

const PAGE_TABLE_ENTRIES: usize = 512;

bitflags::bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct PageFlags: u64 {
        const PRESENT = 1 << 0;
        const WRITEABLE = 1 << 1;
        const USER = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const HUGE_PAGE = 1 << 7;
        const GLOBAL = 1 << 8;
    }
}

impl PageFlags {
    fn kernel_flags() -> Self {
        Self::PRESENT | Self::WRITEABLE
    }

    fn user_flags() -> Self {
        Self::PRESENT | Self::WRITEABLE | Self::USER
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    fn new(addr: PhysAddr, flags: PageFlags) -> Self {
        let addr: u64 = addr.into();
        Self(addr | flags.bits())
    }

    pub fn set(&mut self, addr: PhysAddr, flags: PageFlags) {
        *self = PageTableEntry::new(addr, flags);
    }

    pub fn get_flags(&self) -> PageFlags {
        PageFlags::from_bits_truncate(self.0)
    }

    pub fn set_flags(&mut self, flags: PageFlags) {
        *self = PageTableEntry::new(self.get_addr(), flags);
    }

    pub fn get_addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & 0x000f_ffff_ffff_f000)
    }

    pub fn set_addr(&mut self, addr: PhysAddr) {
        *self = PageTableEntry::new(addr, self.get_flags());
    }
}

impl core::fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "[addr={:?}, flags={:?}]",
            self.get_addr(),
            self.get_flags()
        )
    }
}

#[repr(transparent)]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    /// Get or allocate the child page table referenced by `entries[idx]`.
    /// Intermediate tables always have PRESENT | WRITEABLE | USER flags.
    fn get_or_alloc_child(&mut self, idx: usize) -> &mut PageTable {
        if !self.entries[idx].get_flags().contains(PageFlags::PRESENT) {
            let frame = unsafe {
                FRAME_ALLOCATOR.lock().alloc_block(1)
                    .expect("Out of frames for page table")
            };
            self.entries[idx].set(
                frame,
                PageFlags::PRESENT | PageFlags::WRITEABLE | PageFlags::USER,
            );
        }
        let addr = self.entries[idx].get_addr();
        unsafe { &mut *(addr.as_mut_ptr::<PageTable>()) }
    }

    /// Map `num_pages` pages starting at `virt_addr`.
    /// If `kernel` is true, creates a 1:1 identity mapping (no frame allocation at leaf level).
    /// If `kernel` is false, allocates fresh frames for each leaf page.
    /// Page 0 (address 0x0) is always left not-present (null pointer guard).
    /// Returns the number of pages mapped.
    fn map(&mut self, virt_addr: u64, num_pages: usize, kernel: bool) -> usize {
        let flags = if kernel { PageFlags::kernel_flags() } else { PageFlags::user_flags() };

        for i in 0..num_pages {
            let vaddr = virt_addr + (i as u64) * (PAGE_SIZE as u64);

            let pml4_idx = ((vaddr >> 39) & 0x1FF) as usize;
            let pdpt_idx = ((vaddr >> 30) & 0x1FF) as usize;
            let pd_idx   = ((vaddr >> 21) & 0x1FF) as usize;
            let pt_idx   = ((vaddr >> 12) & 0x1FF) as usize;

            let pdpt = self.get_or_alloc_child(pml4_idx);
            let pd   = pdpt.get_or_alloc_child(pdpt_idx);
            let pt   = pd.get_or_alloc_child(pd_idx);

            if vaddr == 0 {
                pt.entries[pt_idx] = PageTableEntry(0);
            } else if kernel {
                pt.entries[pt_idx].set(PhysAddr::new(vaddr), flags);
            } else {
                let frame = unsafe {
                    FRAME_ALLOCATOR.lock().alloc_block(1)
                        .expect("Out of frames for user page")
                };
                pt.entries[pt_idx].set(frame, flags);
            }
        }
        num_pages
    }
}

pub fn read_cr3() -> &'static mut PageTable {
    let value: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) value);
    }

    unsafe {
        PhysAddr::new(value & 0xffff_ffff_ffff_f000)
            .as_mut_ptr::<PageTable>()
            .as_mut()
            .unwrap()
    }
}

pub unsafe fn write_cr3(pml4: &PageTable) {
    let addr: u64 = ptr::from_ref(pml4) as u64;
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) addr);
    }
}

pub unsafe fn write_cr3_raw(pml4_addr: u64) {
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) pml4_addr);
    }
}

/// Create a new PML4 with the entire physical address space identity-mapped.
/// Page 0 is not-present to catch null pointer dereferences.
pub fn init_kernel_tables() -> &'static mut PageTable {
    let max_phys_addr = FRAME_ALLOCATOR.lock().get_max_phys_addr();
    let num_pages = (max_phys_addr.raw() as usize + PAGE_SIZE - 1) / PAGE_SIZE;

    unsafe {
        let pml4 = FRAME_ALLOCATOR.lock()
                .alloc_block(1)
                .expect("Failed to allocate frame for PML4!")
                .as_mut_ptr::<PageTable>()
                .as_mut()
                .unwrap();

        pml4.map(0, num_pages, true);
        pml4
    }
}

/// Map the top page of the user stack into the given PML4 and return the stack top pointer.
/// Only the topmost page is initially mapped; additional pages are allocated on demand
/// by the page fault handler when the stack grows beyond one page.
/// The VMA still covers the full USER_STACK_VIRT_START..USER_STACK_VIRT_END range.
pub unsafe fn map_user_stack(pml4_table: &mut PageTable) -> *mut u8 {
    let top_page_start = (USER_STACK_VIRT_END - PAGE_SIZE) as u64;
    pml4_table.map(top_page_start, 1, false);
    USER_STACK_VIRT_END as *mut u8
}

/// Check if a page fault address falls within the user stack VMA and grow if needed.
/// Returns true if the stack was successfully grown, false otherwise.
pub fn check_and_grow_user_stack(fault_addr: u64) -> bool {
    let stack_start = USER_STACK_VIRT_START as u64;
    let stack_end = USER_STACK_VIRT_END as u64;

    if fault_addr < stack_start || fault_addr >= stack_end {
        return false;
    }

    let page_start = fault_addr & !(PAGE_SIZE as u64 - 1);
    let pml4 = read_cr3();
    pml4.map(page_start, 1, false);
    true
}

/// Map a user application binary into the given PML4 at USER_CODE_VIRT_START.
/// Allocates physical frames, copies the app data, and creates the virtual mapping.
pub unsafe fn map_user_app(pml4_table: &mut PageTable, app_data: &[u8]) {
    let num_pages = (app_data.len() + PAGE_SIZE - 1) / PAGE_SIZE;

    let phys_base = {
        FRAME_ALLOCATOR.lock().alloc_block(num_pages)
            .expect("Out of frames for user app")
    };

    unsafe {
        let dest = phys_base.as_mut_ptr::<u8>();
        core::ptr::copy_nonoverlapping(app_data.as_ptr(), dest, app_data.len());
        let remaining = num_pages * PAGE_SIZE - app_data.len();
        if remaining > 0 {
            core::ptr::write_bytes(dest.add(app_data.len()), 0, remaining);
        }
    }

    let virt_start = USER_CODE_VIRT_START as u64;
    let flags = PageFlags::user_flags();
    for i in 0..num_pages {
        let vaddr = virt_start + (i as u64) * (PAGE_SIZE as u64);
        let paddr = PhysAddr::new(phys_base.raw() + (i as u64) * (PAGE_SIZE as u64));

        let pml4_idx = ((vaddr >> 39) & 0x1FF) as usize;
        let pdpt_idx = ((vaddr >> 30) & 0x1FF) as usize;
        let pd_idx   = ((vaddr >> 21) & 0x1FF) as usize;
        let pt_idx   = ((vaddr >> 12) & 0x1FF) as usize;

        let pdpt = pml4_table.get_or_alloc_child(pml4_idx);
        let pd   = pdpt.get_or_alloc_child(pdpt_idx);
        let pt   = pd.get_or_alloc_child(pd_idx);

        pt.entries[pt_idx].set(paddr, flags);
    }
}

/// Map a user heap region into the given PML4.
/// Allocates physical frames and maps them with user flags at the given virtual address.
pub unsafe fn map_user_heap(pml4_table: &mut PageTable, heap_start: u64, num_pages: usize) {
    pml4_table.map(heap_start, num_pages, false);
}
