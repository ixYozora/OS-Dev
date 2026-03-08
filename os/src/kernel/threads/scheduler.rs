/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: scheduler                                                       ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: A basic round-robin scheduler for cooperative threads.          ║
   ║         No priorities supported.                                        ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Autor:  Michael Schoettner, 15.05.2023                                  ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt::Display;
use core::{fmt, ptr};
use core::sync::atomic::{AtomicBool, AtomicUsize};
use spin::{Mutex, Once};
use crate::kernel::{allocator, cpu};
use crate::kernel::cpu::enable_int_nested;
use crate::kernel::processes::process::{self, Process};
use crate::kernel::processes::vma::{VMA, VmaType};
use crate::kernel::multiboot::MULTIBOOT_INFO;
use crate::consts::{USER_CODE_VIRT_START, USER_STACK_VIRT_START, USER_STACK_VIRT_END, PAGE_SIZE};
use crate::kernel::threads::idle_thread::idle_thread;
use crate::kernel::threads::thread;
use crate::kernel::threads::thread::Thread;
use crate::library::queue::LinkedQueue;

/// Global scheduler instance
static SCHEDULER: Once<Scheduler> = Once::new();

/// Global access to the scheduler.
pub fn get_scheduler() -> &'static Scheduler {
    SCHEDULER.call_once(|| { Scheduler::new() })
}

/// Global flag to indicate if the scheduler is active.
pub static SCHEDULER_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Unlock the scheduler state.
/// This function is called from assembly code.
/// Usually, the mutex would be unlocked automatically when going out of scope.
/// However, since we switch to a different thread in `yield_cpu()` and `exit()`,
/// the scope is not left and the mutex remains locked.
/// As a workaround, we provide this function to unlock the scheduler manually.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unlock_scheduler() {
    unsafe {
        get_scheduler().state.force_unlock();
    }
}

/// The state of the scheduler.
/// It contains the active thread and the ready queue with all other threads.
/// The state is contained in its own struct so that it can be locked via a mutex.
struct SchedulerState {
    active_thread: Option<Box<Thread>>,
    ready_queue: LinkedQueue<Box<Thread>>,
}

/// Represents the scheduler.
/// It is round-robin-based and uses a queue to manage the threads.
pub struct Scheduler {
    state: Mutex<SchedulerState>,
}

impl Scheduler {
    /// Create a new scheduler instance with an empty ready queue
    /// and an idle thread as the active thread.
    pub fn new() -> Self {
        let state = SchedulerState {
            active_thread: Some(Thread::new_kernel_thread(idle_thread)),
            ready_queue: LinkedQueue::new(),
            //initialized: false,
        };

        Scheduler { state:  Mutex::new(state) }
    }

    /// Get the ID of the currently active thread.
    pub fn get_active_tid(&self) -> usize {
        let state = self.state.lock();
        state.active_thread.as_ref().unwrap().get_id()
    }

    /// Get the process ID of the currently active thread.
    pub fn get_active_pid(&self) -> usize {
        let state = self.state.lock();
        state.active_thread.as_ref().unwrap().get_pid()
    }

    /// Spawn a new process: create a Process, a user thread, and register both.
    /// Also creates initial VMAs for Code and Stack regions.
    /// Spawn a new process. Returns the PID on success, or 0 if the app was not found.
    pub fn spawn_process(&self, app_name: &str) -> usize {
        let app_size = {
            let multiboot = MULTIBOOT_INFO.get().expect("Multiboot info not available");
            let archive = multiboot.get_initrd_archive().expect("No initrd archive found");
            let mut size = 0usize;
            for entry in archive.entries() {
                if let Ok(name) = entry.filename().as_str() {
                    if name == app_name {
                        size = entry.data().len();
                        break;
                    }
                }
            }
            size
        };

        if app_size == 0 {
            return 0;
        }

        let proc = Process::new(app_name);
        let pid = proc.get_id();
        process::add_process(proc);

        let mut thread = Thread::new_user_thread(app_name);
        thread.set_pid(pid);

        let code_end = USER_CODE_VIRT_START as u64
            + ((app_size + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE) as u64;
        let code_vma = VMA::new(USER_CODE_VIRT_START as u64, code_end, VmaType::Code);
        process::add_vma(pid, code_vma).expect("Failed to add Code VMA");

        let stack_vma = VMA::new(USER_STACK_VIRT_START as u64, USER_STACK_VIRT_END as u64, VmaType::Stack);
        process::add_vma(pid, stack_vma).expect("Failed to add Stack VMA");

        self.ready(thread);
        pid
    }

    /// Start the scheduler.
    /// This function must only be called once.
    pub fn schedule(&self) {
        let mut state = self.state.lock();
        //state.initialized = true;
        SCHEDULER_ACTIVE.store(true, core::sync::atomic::Ordering::Relaxed);
        state.active_thread.as_mut().unwrap().start();

    }

    /// Register a new thread in the ready queue.
    pub fn ready(&self, thread: Box<Thread>) {
        let mut state = self.state.lock();

        state.ready_queue.enqueue(thread);
    }

    pub fn ready_after_block(&self, thread: Box<Thread>) {
        let mut state = self.state.lock();
        state.ready_queue.enqueue(thread);
    }

    /// Terminate the current (calling) thread and switch to the next one.
    /// Also removes the associated process from the process table.
    pub fn exit(&self) {
        let mut state = self.state.lock();

        let mut current = state.active_thread.take().unwrap();
        let pid = current.get_pid();
        if pid != 0 {
            process::remove_process(pid);
        }

        let next = state.ready_queue.dequeue().unwrap();
        state.active_thread = Some(next);

        unsafe {
            Thread::switch(current.as_mut(), state.active_thread.as_mut().unwrap().as_mut());
        }
    }

    /// Yield the CPU and switch to the next thread in the ready queue.
    pub fn yield_cpu(&self) {
        if let Some(mut state) = self.state.try_lock() {
            // Must be inited and not locked.
            if !SCHEDULER_ACTIVE.load(core::sync::atomic::Ordering::Relaxed) {
                return;
            }

            if allocator::is_locked() {
                return;
            }

            let mut current = state.active_thread.take().unwrap();
            let current_ptr = current.as_mut() as *mut Thread;
            if let Some(next) = state.ready_queue.dequeue() {
                state.active_thread = Some(next);
                state.ready_queue.enqueue(current);
                unsafe {
                    Thread::switch(current_ptr, state.active_thread.as_mut().unwrap().as_mut());
                }
            } else {
                state.active_thread = Some(current);
            }
        }

    }

    /// Kill the thread with the given ID by removing it from the ready queue.
    pub fn kill(&self, to_kill_id: usize) {

        let mut state = self.state.lock();

        if let Some(active) = state.active_thread.as_mut() {
            if active.get_id() == to_kill_id {
                self.exit();
                return;
            }
        }
        state.ready_queue.remove(|thread| thread.get_id() == to_kill_id);

    }

    /// Check if the scheduler state is currently locked.
    pub fn is_locked(&self) -> bool {
        self.state.is_locked()
    }

    /// Prepare the current thread for blocking.
    /// This functions disables interrupts and return the current thread,
    /// as well as the return value from `cpu::disable_int_nested()`.
    /// To complete the blocking operation call `switch_from_blocked_thread()`,
    /// which will enable interrupts again and resume the scheduler.
    pub fn prepare_block(&self) -> (Box<Thread>, bool) {

        let interrupts_enabled = cpu::disable_int_nested();
        let mut state = self.state.lock();
        let current_thread = state.active_thread.take().unwrap();
        (current_thread, interrupts_enabled)

    }

    /// Complete a blocking operation begun with `prepare_block()`.
    /// This resumes the scheduler and switches to the next thread in the ready queue.
    pub unsafe fn switch_from_blocked_thread(&self, blocked_thread: *mut Thread, interrupts_enabled: bool) {

        let mut state = self.state.lock();

        if let Some(next_thread) = state.ready_queue.dequeue() {

            state.active_thread = Some(next_thread);

            unsafe {
                Thread::switch(blocked_thread, state.active_thread.as_mut().unwrap().as_mut());
            }
            enable_int_nested(interrupts_enabled);
        } else {
            unsafe {
                state.active_thread = Some(Box::from_raw(blocked_thread));
            }
            drop(state);
            enable_int_nested(interrupts_enabled);
            self.yield_cpu();
        }


    }
}

impl Display for Scheduler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let state = self.state.lock();
        let active = state.active_thread.as_ref().unwrap();

        write!(f, "active: {}, ready: {}", active, state.ready_queue)
    }
}