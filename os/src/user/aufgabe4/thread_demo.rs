use crate::devices::buff_print::{lfb_print, lfb_set_color, lfb_clear, lfb_get_cursor_pos, lfb_set_cursor_pos};
use crate::devices::lfb::{HHU_GREEN, WHITE, BLUE, YELLOW, RED, HHU_BLUE};
use crate::kernel::threads::scheduler::{get_scheduler, Scheduler};
use crate::kernel::threads::thread::Thread;
use crate::user::aufgabe4::hello_world_thread;
use crate::lfb_print;

// Global state without additional synchronization
static mut THREAD_COUNTERS: [u32; 3] = [0, 0, 0];
static mut THREAD_ACTIVE: [bool; 3] = [true, true, true];
static mut CONTROL_THREAD_ID: usize = 0;
static mut UPDATE_COUNTER: u32 = 0;

// Add static variables to store cursor position
static mut COUNTER_START_X: u32 = 0;
static mut COUNTER_START_Y: u32 = 0;

fn thread_entry() {
    let scheduler = get_scheduler();
    let my_id = scheduler.get_active_tid();
    let mut is_control_thread = false;
    let mut thread_index = 3;

    unsafe {
        if THREAD_ACTIVE[0] && my_id == CONTROL_THREAD_ID {
            is_control_thread = true;
            thread_index = 0;
        } else if THREAD_ACTIVE[1] && my_id == CONTROL_THREAD_ID + 1 {
            thread_index = 1;
        } else if THREAD_ACTIVE[2] && my_id == CONTROL_THREAD_ID + 2 {
            thread_index = 2;
        }
    }

    if thread_index < 3 {
        loop {
            unsafe {
                if !THREAD_ACTIVE[thread_index] {
                    break;
                }

                THREAD_COUNTERS[thread_index] += 1;

                if thread_index == 0 && THREAD_COUNTERS[thread_index] % 100 == 0 {
                    // One-time setup
                    if THREAD_COUNTERS[thread_index] == 100 {
                        lfb_set_color(WHITE);
                        lfb_print!("=== Thread Demo - Test: Three Counters ===\n\n");

                        let (x, y) = lfb_get_cursor_pos();
                        COUNTER_START_X = x;
                        COUNTER_START_Y = y;
                    }

                    // Clear only the line where counters live, then redraw
                    {
                        let mut lfb = crate::devices::lfb::get_lfb().lock();
                        lfb.clear_text_line_from(COUNTER_START_X, COUNTER_START_Y);
                    }
                    lfb_set_cursor_pos(COUNTER_START_X, COUNTER_START_Y);

                    // Draw counters
                    lfb_set_color(YELLOW);
                    lfb_print!("[0] ");
                    lfb_set_color(HHU_GREEN);
                    lfb_print!("{:05}   ", THREAD_COUNTERS[0]);

                    lfb_set_color(BLUE);
                    lfb_print!("[1] ");
                    lfb_set_color(HHU_GREEN);
                    lfb_print!("{:05}   ", THREAD_COUNTERS[1]);

                    lfb_set_color(RED);
                    lfb_print!("[2] ");
                    lfb_set_color(HHU_GREEN);
                    lfb_print!("{:05}", THREAD_COUNTERS[2]);
                }

                // Control thread logic
                if is_control_thread {
                    if THREAD_COUNTERS[thread_index] == 5000 {
                        THREAD_ACTIVE[1] = false;
                        THREAD_ACTIVE[2] = false;

                        lfb_set_cursor_pos(0, COUNTER_START_Y + 16);
                        lfb_set_color(RED);
                        lfb_print!(">>> Killed threads 1 & 2\n");
                    }

                    if THREAD_COUNTERS[thread_index] == 10000 {
                        THREAD_ACTIVE[0] = false;
                        lfb_set_cursor_pos(0, COUNTER_START_Y + 32);
                        lfb_set_color(RED);
                        lfb_print!(">>> Demo complete!\n");

                        // redraw prompt for shell
                        lfb_set_color(HHU_BLUE);
                        lfb_print!("yozora$ ");
                        lfb_set_color(WHITE);

                        for _ in 0..10 { scheduler.yield_cpu(); }
                        scheduler.exit();
                    }
                }
            }
            scheduler.yield_cpu();
        }
    }
}

pub fn run() {
    lfb_print!("Starting Thread Demo\n");

    let counter_thread = Thread::new(thread_entry);
    let scheduler = get_scheduler();
    scheduler.ready(counter_thread);

    lfb_print!("Running Test: Three Counters with Kill/Exit\n\n");

    // Test 2: Three counter threads
    unsafe {
        // Reset state
        THREAD_COUNTERS = [0, 0, 0];
        THREAD_ACTIVE = [true, true, true];
    }

    let threads = [
        Thread::new(thread_entry),
        Thread::new(thread_entry),
        Thread::new(thread_entry),
    ];

    unsafe {
        CONTROL_THREAD_ID = threads[0].get_id();
    }

    for thread in threads {
        scheduler.ready(thread);
    }

    // Run test 2
    loop {
        scheduler.yield_cpu();
    }
}