use crate::devices::buff_print::{buff_set_color, buff_get_cursor_pos, buff_set_cursor_pos};
use crate::devices::lfb::{HHU_GREEN, WHITE, BLUE, YELLOW, RED, HHU_BLUE};
use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::threads::thread::Thread;
use crate::library::mutex::Mutex;
// Uncomment the line below to test with Spinlock instead
//use crate::library::spinlock::Spinlock as Mutex;
use alloc::vec::Vec;
use crate::buff_print;
// Shared data without turn-based logic
struct SharedData {
    counters: [u32; 3],
    active: bool,
    display_counter: u32,
    control_thread_id: usize,
    counter_start_x: u32,
    counter_start_y: u32,
    demo_initialized: bool,
    last_active_thread: usize,
    thread_switches: Vec<usize>, // Track which threads ran
}

static SHARED_DATA: Mutex<SharedData> = Mutex::new(SharedData {
    counters: [0, 0, 0],
    active: true,
    display_counter: 0,
    control_thread_id: 0,
    counter_start_x: 0,
    counter_start_y: 0,
    demo_initialized: false,
    last_active_thread: 999, // Invalid initial value
    thread_switches: Vec::new(),
});

fn competitive_thread_entry() {
    let scheduler = get_scheduler();
    let my_id = scheduler.get_active_tid();
    let thread_index;

    // Get thread index
    {
        let state = SHARED_DATA.lock();
        thread_index = (my_id - state.control_thread_id) as usize;
    }

    if thread_index >= 3 {
        return;
    }

    loop {
        // All threads compete for the same lock - no turn-based logic!
        {
            let mut data = SHARED_DATA.lock();

            if !data.active {
                break;
            }

            // One-time setup
            if !data.demo_initialized && thread_index == 0 {
                let (x, y) = buff_get_cursor_pos();
                data.counter_start_x = x;
                data.counter_start_y = y;
                buff_set_color(WHITE);
                buff_print!("=== Lock Competition Demo ===\n");
                buff_print!("All threads compete for the same lock\n\n");
                data.demo_initialized = true;
            }

            // Increment this thread's counter
            data.counters[thread_index] += 1;
            data.display_counter += 1;

            // Track thread switches
            if data.last_active_thread != thread_index {
                if data.thread_switches.len() < 50 { // Limit to avoid memory issues
                    data.thread_switches.push(thread_index);
                }
                data.last_active_thread = thread_index;
            }

            // Update display less frequently for readability
            if data.display_counter % 100 == 0 {
                // Clear and update display
                {
                    let mut lfb = crate::devices::lfb::get_lfb().lock();
                    lfb.clear_text_line(data.counter_start_y);
                    lfb.clear_text_line(data.counter_start_y + 16);
                    lfb.clear_text_line(data.counter_start_y + 32);
                }

                buff_set_cursor_pos(data.counter_start_x, data.counter_start_y);

                // Display counters
                for i in 0..3 {
                    if i == thread_index {
                        buff_set_color(RED); // Highlight currently running thread
                        buff_print!(">>> ");
                    } else {
                        buff_set_color(WHITE);
                        buff_print!("    ");
                    }

                    match i {
                        0 => buff_set_color(YELLOW),
                        1 => buff_set_color(BLUE),
                        2 => buff_set_color(HHU_GREEN),
                        _ => {}
                    }
                    buff_print!("[{}] ", i);

                    buff_set_color(WHITE);
                    buff_print!("{:05}   ", data.counters[i]);
                }

                // Show current running thread
                buff_set_cursor_pos(0, data.counter_start_y + 16);
                buff_set_color(HHU_BLUE);
                buff_print!("Currently running: Thread {}", thread_index);

                // Show recent thread execution pattern
                buff_set_cursor_pos(0, data.counter_start_y + 32);
                buff_set_color(YELLOW);
                buff_print!("Recent pattern: ");
                if data.thread_switches.len() >= 10 {
                    let start = data.thread_switches.len() - 10;
                    for i in start..data.thread_switches.len() {
                        buff_print!("{}", data.thread_switches[i]);
                        if i < data.thread_switches.len() - 1 {
                            buff_print!("->");
                        }
                    }
                }
            }

            // End demo when total work reaches target
            let total_work: u32 = data.counters.iter().sum();
            if total_work >= 15000 {
                data.active = false;

                buff_set_cursor_pos(0, data.counter_start_y + 48);
                buff_set_color(RED);
                buff_print!("=== Demo Complete ===\n");
                buff_set_color(WHITE);

                // Analyze the pattern
                let mut consecutive_count = 1;
                let mut max_consecutive = 1;
                for i in 1..data.thread_switches.len().min(20) {
                    if data.thread_switches[i] == data.thread_switches[i-1] {
                        consecutive_count += 1;
                        max_consecutive = max_consecutive.max(consecutive_count);
                    } else {
                        consecutive_count = 1;
                    }
                }

                buff_set_color(HHU_BLUE);
                buff_print!("yozora$ ");
                buff_set_color(WHITE);
                break;
            }
        } // Lock released here

        // Small yield to allow other threads a chance
        // This is crucial - without yielding, one thread might monopolize
        scheduler.yield_cpu();
    }

    scheduler.exit();
}

pub fn run() {
    buff_set_color(WHITE);
    buff_print!("Starting Lock Competition Demo\n");

    let scheduler = get_scheduler();

    // Reset shared data
    {
        let mut data = SHARED_DATA.lock();
        data.counters = [0, 0, 0];
        data.active = true;
        data.display_counter = 0;
        data.demo_initialized = false;
        data.last_active_thread = 999;
        data.thread_switches.clear();
    }

    // Create competing threads
    let threads = [
        Thread::new_kernel_thread(competitive_thread_entry),
        Thread::new_kernel_thread(competitive_thread_entry),
        Thread::new_kernel_thread(competitive_thread_entry),
    ];

    // Set control thread ID
    {
        let mut data = SHARED_DATA.lock();
        data.control_thread_id = threads[0].get_id();
    }

    // Start all threads
    for thread in threads {
        scheduler.ready(thread);
    }

    // Wait for completion
    loop {
        let done = {
            let data = SHARED_DATA.lock();
            !data.active
        };

        if done {
            break;
        }

        scheduler.yield_cpu();
    }
}
