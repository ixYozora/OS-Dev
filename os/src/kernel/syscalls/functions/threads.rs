use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::processes::process;

pub extern "C" fn sys_thread_yield() {
    get_scheduler().yield_cpu();
}

pub extern "C" fn sys_thread_exit() {
    get_scheduler().exit();
}

pub extern "C" fn sys_thread_get_id() -> u64 {
    get_scheduler().get_active_tid() as u64
}

pub extern "C" fn sys_get_process_id() -> u64 {
    get_scheduler().get_active_pid() as u64
}

pub extern "C" fn sys_spawn_process(name_ptr: *const u8, name_len: usize) -> u64 {
    let name_slice = unsafe { core::slice::from_raw_parts(name_ptr, name_len) };
    let name = match core::str::from_utf8(name_slice) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    get_scheduler().spawn_process(name) as u64
}

pub extern "C" fn sys_wait_pid(pid: u64) {
    let pid = pid as usize;
    while process::process_exists(pid) {
        get_scheduler().yield_cpu();
    }
}
