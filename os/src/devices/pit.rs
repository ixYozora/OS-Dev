/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: pit                                                             ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Programmable Interval Timer.                                    ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author:  Michael Schoettner, HHU, 15.6.2023                             ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use alloc::boxed::Box;
use core::arch::asm;
use core::sync::atomic::AtomicUsize;
use spin::Once;
use crate::devices::cga;
use crate::devices::cga::{Color, CGA, CGA_COLUMNS, CGA_ROWS};
use crate::kernel::cpu;
use crate::kernel::cpu::IoPort;
use crate::kernel::interrupts::{intdispatcher, pic};
use crate::kernel::interrupts::intdispatcher::{INT_VECTORS, InterruptVector};
use crate::kernel::interrupts::isr::ISR;
use crate::kernel::interrupts::pic::{Irq, PIC};
use crate::kernel::threads::scheduler::get_scheduler;
use crate::kernel::allocator::is_locked;

// Ports
const PORT_CTRL: u16 = 0x43;
const PORT_DATA0: u16 = 0x40;

const TIMER_FREQ: usize = 1193182; // Timer frequency in Hz
const NANOSECONDS_PER_TICK: usize = 1_000_000_000 / TIMER_FREQ; // Nanoseconds per timer tick

/// Global timer instance.
/// Not accessible from outside the module.
/// To get the current system time, use `get_system_time()`.
static TIMER: Once<Timer> = Once::new();

/// Global system time in milliseconds.
static SYSTEM_TIME: AtomicUsize = AtomicUsize::new(0);

/// Characters used for the spinner animation.
static SPINNER_CHARS: &[char] = &['|', '/', '-', '\\'];

/// Get the current system time in milliseconds.
pub fn get_system_time() -> usize {
    SYSTEM_TIME.load(core::sync::atomic::Ordering::Relaxed)
}

/// Wait for a specified number of milliseconds using the system time.
pub fn wait(ms: usize) {

    let start = get_system_time();
    while get_system_time() - start < ms {
        continue;
    }

}

/// Returns (hours, minutes, seconds)
pub fn uptime_hms() -> (usize, usize, usize) {
    let total_secs = get_system_time() / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    (hours, minutes, seconds)
}


/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Interrupt service routine implementation.                               ║
   ╚═════════════════════════════════════════════════════════════════════════╝ */

/// Register the timer interrupt handler.
pub fn plugin() {

    TIMER.call_once(|| {
        let mut timer = Timer::new();
        timer.set_interrupt_interval(1); // Set the timer to trigger every 1 ms
        intdispatcher::INT_VECTORS.lock().register(InterruptVector::Pit, Box::new(TimerISR { interval_ms: 1 }));
        PIC.lock().allow(Irq::Timer); // Allow timer interrupts in the PIC
        timer
    });

    //clean look
    SYSTEM_TIME.store(0, core::sync::atomic::Ordering::Relaxed);

}

/// The timer interrupt service routine.
struct TimerISR {
    /// The interval between timer interrupts in milliseconds.
    interval_ms: usize,
}

impl ISR for TimerISR {
    fn trigger(&self) {

        //add 1 to the global system time
        let current_time = SYSTEM_TIME.fetch_add(self.interval_ms, core::sync::atomic::Ordering::Relaxed);

        //careful with interrupts...
        if !get_scheduler().is_locked() && !is_locked() && !cga::CGA.is_queue_locked() {
            //update spinner every 250 ms
            if current_time % 250 == 0 {
                if let Some(mut cga) = CGA.try_lock() {
                    let spinner_index = (current_time / 250) % SPINNER_CHARS.len();
                    let spinner_char = SPINNER_CHARS[spinner_index];
                    //cga.setpos(CGA_COLUMNS - 1, 0);
                    //cga.print_byte(spinner_char as u8);
                    cga.show(CGA_COLUMNS - 1, 0, spinner_char, 4);
                }
            }
        }


        //thread wechsel wegen deadlocks
        unsafe {
            INT_VECTORS.force_unlock();
        }
        // Call the scheduler to switch to the next thread
        get_scheduler().yield_cpu();
    }
}

/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Implementation of the PIT driver itself.                                ║
   ╚═════════════════════════════════════════════════════════════════════════╝ */

/// Represents the programmable interval timer.
struct Timer {
    control_port: IoPort,
    data_port0: IoPort
}

impl Timer {
    /// Create a new Timer instance.
    pub const fn new() -> Timer {
        Timer {
            control_port: IoPort::new(PORT_CTRL),
            data_port0: IoPort::new(PORT_DATA0)
        }
    }

    /// Set the timer interrupt interval in milliseconds.
    pub fn set_interrupt_interval(&mut self, interval_ms: usize) {

        //Verwenden Sie hierfür im PIT den Zähler 0 und Modus 3 und
        // laden Sie den Zähler mit einem passenden Wert, sodass der PIT jede Millisekunde ein Interrupt ausgelöst.
        // 0x36, da counter 0, RW auf 11 wieder, mode ist 3 also 011 und 0 wieder am ende
        let divisor = TIMER_FREQ / 1000 / interval_ms; // Timer auf ms

        unsafe {
            self.control_port.outb(0x36);
            self.data_port0.outb((divisor & 0xFF) as u8); // LSB
            self.data_port0.outb(((divisor >> 8) & 0xFF) as u8); // MSB
        }
    }
}