use alloc::format;
use crate::kernel::syscalls::user_api::*;
use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::threads::thread::Thread;

pub fn paging_test() {
    let t1 = Thread::new_user_thread(user_thread_a);
    let t2 = Thread::new_user_thread(user_thread_b);

    let scheduler = get_scheduler();
    scheduler.ready(t1);
    scheduler.ready(t2);
    scheduler.schedule();
}

fn user_thread_a() {
    let tid = usr_thread_get_id();
    let mut counter: u64 = 0;
    loop {
        let msg = format!("Thread A (TID {}): counter = {}\n", tid, counter);
        usr_print(&msg);
        counter += 1;
        usr_thread_yield();
    }
}

fn user_thread_b() {
    let tid = usr_thread_get_id();
    let mut counter: u64 = 0;
    loop {
        let msg = format!("Thread B (TID {}): counter = {}\n", tid, counter);
        usr_print(&msg);
        counter += 1;
        usr_thread_yield();
    }
}
