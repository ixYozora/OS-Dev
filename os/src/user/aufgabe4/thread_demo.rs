use crate::devices::cga;
use crate::devices::cga::Color;
use crate::devices::cga::CGA;
use crate::kernel::threads::scheduler::{get_scheduler, Scheduler};
use crate::kernel::threads::thread::Thread;
use crate::user::aufgabe4::hello_world_thread;


const COUNTER_POS: (usize, usize) = (10, 10);
const THREAD_POSITIONS: [(usize, usize); 3] = [(10, 5), (30, 5), (50, 5)];

// Global state without additional synchronization
static mut THREAD_COUNTERS: [u32; 3] = [0, 0, 0];
static mut THREAD_ACTIVE: [bool; 3] = [true, true, true];
static mut CONTROL_THREAD_ID: usize = 0;

fn thread_entry() {
    let scheduler = get_scheduler();
    let my_id = scheduler.get_active_tid();

    // Default to simple counter behavior
    let mut counter = 0;
    let mut is_control_thread = false;
    let mut thread_index = 0;

    // Determine if this is one of the three counter threads
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
        // Three counter thread behavior
        let (x, y) = THREAD_POSITIONS[thread_index];
        let attr_yellow = CGA.lock().attribute(Color::Black, Color::Yellow, false);
        let attr_green = CGA.lock().attribute(Color::Black, Color::Green, false);

        loop {
            unsafe {
                if !THREAD_ACTIVE[thread_index] {
                    break; // Terminate if killed
                }

                {
                    let mut cga = CGA.lock();
                    cga.setpos(x, y);
                    cga.show(x, y, '[', attr_yellow);
                    cga.show(x + 1, y, char::from_digit(thread_index as u32, 10).unwrap_or('?'), attr_yellow);
                    cga.show(x + 2, y, ']', attr_yellow);

                    // Format counter with 5 digits
                    let digits = [
                        (THREAD_COUNTERS[thread_index] / 10000) % 10,
                        (THREAD_COUNTERS[thread_index] / 1000) % 10,
                        (THREAD_COUNTERS[thread_index] / 100) % 10,
                        (THREAD_COUNTERS[thread_index] / 10) % 10,
                        THREAD_COUNTERS[thread_index] % 10,
                    ];

                    for (i, digit) in digits.iter().enumerate() {
                        cga.show(x + 4 + i, y, char::from_digit(*digit as u32, 10).unwrap_or('?'), attr_green);
                    }
                }

                THREAD_COUNTERS[thread_index] += 1;

                // Control thread logic
                if is_control_thread {
                    // Kill other threads after 10000 counts
                    if THREAD_COUNTERS[thread_index] == 10000 {
                        THREAD_ACTIVE[1] = false;
                        THREAD_ACTIVE[2] = false;
                    }

                    // Exit after 30000 counts
                    if THREAD_COUNTERS[thread_index] == 30000 {
                        THREAD_ACTIVE[0] = false;
                        scheduler.exit();
                    }
                }
            }

            scheduler.yield_cpu();
        }
    } else {
        // Simple counter behavior
        let attr_yellow = CGA.lock().attribute(Color::Black, Color::Yellow, false);
        let attr_green = CGA.lock().attribute(Color::Black, Color::Green, false);

        loop {
            {
                let mut cga = CGA.lock();
                cga.setpos(COUNTER_POS.0, COUNTER_POS.1);
                cga.show(COUNTER_POS.0, COUNTER_POS.1, '[', attr_yellow);
                cga.show(COUNTER_POS.0 + 1, COUNTER_POS.1,
                         char::from_digit(counter % 10, 10).unwrap_or('?'),
                         attr_green);
                cga.show(COUNTER_POS.0 + 2, COUNTER_POS.1, ']', attr_yellow);
            }
            counter += 1;
            scheduler.yield_cpu();
        }
    }
}

pub fn run() {
    kprintln!("Starting Thread Demo");

    // Test 1: Hello World thread + simple counter
    kprintln!("Running Test 1: Hello World + Counter");
    let hello_thread = Thread::new(hello_world_thread::hello_world);
    let counter_thread = Thread::new(thread_entry);

    let scheduler = get_scheduler();
    scheduler.ready(hello_thread);
    scheduler.ready(counter_thread);

    // Let test 1 run
    for _ in 0..1000 {
        scheduler.yield_cpu();
    }

    // Clear screen between tests
    {
        let mut cga = CGA.lock();
        cga.clear();
        cga.setpos(0, 0);
    }

    kprintln!("Running Test 2: Three Counters with Kill/Exit");

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