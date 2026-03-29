use crate::kernel::processes::process;
use crate::kernel::processes::vma::{VMA, VmaType};
use crate::kernel::paging::pages;
use crate::kernel::threads::scheduler::get_scheduler;
use crate::consts::PAGE_SIZE;

pub extern "C" fn sys_dump_vmas() {
    let pid = get_scheduler().get_active_pid();
    process::dump_vmas(pid);
}

pub extern "C" fn sys_map_heap(heap_start: u64, heap_size: u64) {
    let pid = get_scheduler().get_active_pid();
    let pml4 = pages::read_cr3();

    let num_pages = (heap_size as usize + PAGE_SIZE - 1) / PAGE_SIZE;
    unsafe {
        pages::map_user_heap(pml4, heap_start, num_pages);
    }

    let heap_vma = VMA::new(heap_start, heap_start + (num_pages * PAGE_SIZE) as u64, VmaType::Heap);
    process::add_vma(pid, heap_vma).expect("Failed to add Heap VMA");
}
