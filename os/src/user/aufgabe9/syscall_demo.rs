use alloc::string::String;
use alloc::format;
use crate::kernel::syscalls::user_api::*;
use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::threads::thread::Thread;

pub fn syscall_test() {
    let t1 = Thread::new_user_thread(syscall_test_thread);
    let scheduler = get_scheduler();
    scheduler.ready(t1);
    scheduler.schedule();
}

fn syscall_test_thread() {
    usr_hello_world();

    let tid = usr_thread_get_id();
    let msg = format!("Thread ID: {}\n\n", tid);
    usr_print(&msg);

    let mut line = String::new();
    loop {
        let c = usr_get_char();
        if c == '\r' {
            let time_ms = usr_get_system_time();
            let time_s = time_ms / 1000;
            let output = format!(
                "\nYou typed: '{}'\nSystem time: {}s\n\n",
                line, time_s
            );
            usr_print(&output);
            line.clear();
        } else {
            line.push(c);
            let echo = format!("{}", c);
            usr_print(&echo);
        }
    }
}

/// Separate test using fast syscalls (syscall/sysret) — can be spawned additionally.
#[allow(dead_code)]
pub fn fast_syscall_test() {
    let t = Thread::new_user_thread(fast_syscall_test_thread);
    let scheduler = get_scheduler();
    scheduler.ready(t);
    scheduler.schedule();
}

fn fast_syscall_test_thread() {
    fast_usr_hello_world();

    let tid = fast_usr_thread_get_id();
    let msg = format!("Thread ID: {} (fast)\n\n", tid);
    fast_usr_print(&msg);

    let mut line = String::new();
    loop {
        let c = fast_usr_get_char();
        if c == '\r' {
            let time_ms = fast_usr_get_system_time();
            let time_s = time_ms / 1000;
            let output = format!(
                "\nYou typed: '{}'\nSystem time: {}s\n\n",
                line, time_s
            );
            fast_usr_print(&output);
            line.clear();
        } else {
            line.push(c);
            let echo = format!("{}", c);
            fast_usr_print(&echo);
        }
    }
}
