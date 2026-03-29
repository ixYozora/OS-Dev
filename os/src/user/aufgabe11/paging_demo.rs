use crate::kernel::threads::scheduler::get_scheduler;

pub fn paging_test() {
    let scheduler = get_scheduler();
    scheduler.spawn_process("hello");
    scheduler.schedule();
}
