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
use core::sync::atomic::AtomicUsize;
use spin::{Mutex, Once};
use crate::kernel::allocator;
use crate::kernel::threads::idle_thread::idle_thread;
use crate::kernel::threads::thread;
use crate::kernel::threads::thread::Thread;
use crate::library::queue::LinkedQueue;
use crate::cpu;

/// Global scheduler instance
static SCHEDULER: Once<Scheduler> = Once::new();

/// Global access to the scheduler.
pub fn get_scheduler() -> &'static Scheduler {
    SCHEDULER.call_once(|| { Scheduler::new() })
}

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
    initialized: bool,
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
            active_thread: Some(Thread::new(idle_thread)),
            ready_queue: LinkedQueue::new(),
            initialized: false,
        };

        Scheduler { state:  Mutex::new(state) }
    }

    /// Get the ID of the currently active thread.
    pub fn get_active_tid(&self) -> usize {
        let state = self.state.lock();

        state.active_thread.as_ref().unwrap().get_id()
    }

    /// Start the scheduler.
    /// This function must only be called once.
    pub fn schedule(&self) {
        let mut state = self.state.lock();
        state.initialized = true;
        state.active_thread.as_mut().unwrap().start();

    }

    /// Register a new thread in the ready queue.
    pub fn ready(&self, thread: Box<Thread>) {
        let mut state = self.state.lock();

        state.ready_queue.enqueue(thread);
    }

    /// Terminate the current (calling) thread and switch to the next one.
    pub fn exit(&self) {
        let mut state = self.state.lock();

        // The active thread is never None, since we must at least have the idle thread.
        let mut current = state.active_thread.take().unwrap();
        // The idle thread never exits, so there must be at least one thread in the queue.
        let next = state.ready_queue.dequeue().unwrap();

        // Set the dequeued thread as the active thread,
        // overwriting the current one, which we want to exit.
        state.active_thread = Some(next);

        unsafe {
            // Switch to the next thread.
            // `current` still contains the old thread we want to exit,
            // while `state.active_thread` contains the next one.
            Thread::switch(current.as_mut(), state.active_thread.as_mut().unwrap().as_mut());
        }
    }

    /// Yield the CPU and switch to the next thread in the ready queue.
    pub fn yield_cpu(&self) {
        if let Some(mut state) = self.state.try_lock() {
            // Must be inited and not locked.
            if !state.initialized {
                return;
            }

            if allocator::is_locked() {
                return;
            }

            let mut current = state.active_thread.take().unwrap();
            //safe pointer because cant access after putting back in queue
            let current_ptr = current.as_mut() as *mut Thread;

            //put active thread back in ready queue and get next thread
            if let Some(next) = state.ready_queue.dequeue() {
                state.active_thread = Some(next);
                state.ready_queue.enqueue(current);
                unsafe {
                    Thread::switch(current_ptr, state.active_thread.as_mut().unwrap().as_mut());
                }
            } else {
                // If there are no threads in the queue, we just keep the current thread.
                state.active_thread = Some(current);
            }
        }
    }

    /// Kill the thread with the given ID by removing it from the ready queue.
    pub fn kill(&self, to_kill_id: usize) {

        let mut state = self.state.lock();

        // If the thread to kill is the active thread, we exit it.
        if let Some(active) = state.active_thread.as_mut() {
            if active.get_id() == to_kill_id {
                self.exit();
                return;
            }
        }

        // Otherwise, we search for the thread in the ready queue and remove it.
        state.ready_queue.remove(|thread| thread.get_id() == to_kill_id);

    }

    /// Check if the scheduler state is currently locked.
    pub fn is_locked(&self) -> bool {
        self.state.is_locked()
    }

    pub fn is_initialized(&self) -> bool {


        let state = self.state.lock();


        state.initialized


    }

    /// Prepare the current thread for blocking.
    /// This functions disables interrupts and return the current thread,
    /// as well as the return value from `cpu::disable_int_nested()`.
    /// To complete the blocking operation call `switch_from_blocked_thread()`,
    /// which will enable interrupts again and resume the scheduler.
    pub fn prepare_block(&self) -> (Box<Thread>, bool) {

        let mut state = self.state.lock();

        // Must be inited and not locked.
        if !state.initialized {
            panic!("Scheduler not initialized");
        }

        if allocator::is_locked() {
            panic!("Allocator is locked, cannot block");
        }

        let interrupts_enabled = cpu::disable_int_nested();
        let current_thread = state.active_thread.take().unwrap();

        (current_thread, interrupts_enabled)

    }

    /// Complete a blocking operation begun with `prepare_block()`.
    /// This resumes the scheduler and switches to the next thread in the ready queue.
    pub unsafe fn switch_from_blocked_thread(&self, blocked_thread: *mut Thread, interrupts_enabled: bool) {

        let mut state = self.state.lock();

        if let Some(mut next_thread) = state.ready_queue.dequeue() {

            state.active_thread = Some(next_thread);

            unsafe {
                Thread::switch(blocked_thread, state.active_thread.as_mut().unwrap().as_mut());
            }

        } else {
            unsafe {
                state.active_thread = Some(Box::from_raw(blocked_thread));
            }
        }

        cpu::enable_int_nested(interrupts_enabled);

    }

}



impl Display for Scheduler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let state = self.state.lock();
        let active = state.active_thread.as_ref().unwrap();

        write!(f, "active: {}, ready: {}", active, state.ready_queue)
    }
}