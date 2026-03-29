#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{
    usr_clear_text_bands, usr_get_text_cursor, usr_print, usr_set_color, usr_set_text_cursor,
    usr_thread_exit, usr_thread_yield,
};

#[global_allocator]
static _A: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

const fn make_color(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

const WHITE: u32 = make_color(170, 170, 170);
const RED: u32 = make_color(170, 0, 0);
const YELLOW: u32 = make_color(170, 170, 0);
const BLUE: u32 = make_color(0, 0, 170);
const HHU_GREEN: u32 = make_color(151, 191, 13);
const HHU_BLUE: u32 = make_color(0, 106, 179);

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_set_color(WHITE);
    usr_print("Starting Lock Competition Demo\n");

    let mut counters = [0u32, 0u32, 0u32];
    let mut display_counter: u32 = 0;
    let mut last_active: usize = 999;
    let mut switches = [0usize; 50];
    let mut switch_len: usize = 0;
    let mut demo_initialized = false;
    let mut counter_start_x: u32 = 0;
    let mut counter_start_y: u32 = 0;
    let mut active = true;

    while active {
        if !demo_initialized {
            let packed = usr_get_text_cursor();
            counter_start_x = (packed >> 32) as u32;
            counter_start_y = (packed & 0xffff_ffff) as u32;
            usr_set_color(WHITE);
            usr_print("=== Lock Competition Demo ===\n");
            usr_print("All threads compete for the same lock\n\n");
            demo_initialized = true;
        }

        let thread_index = (display_counter as usize) % 3;

        counters[thread_index] = counters[thread_index].saturating_add(1);
        display_counter = display_counter.wrapping_add(1);

        if last_active != thread_index {
            if switch_len < switches.len() {
                switches[switch_len] = thread_index;
                switch_len += 1;
            }
            last_active = thread_index;
        }

        if display_counter % 100 == 0 {
            usr_clear_text_bands(counter_start_y, 16, 3);

            usr_set_text_cursor(counter_start_x, counter_start_y);
            for i in 0..3 {
                if i == thread_index {
                    usr_set_color(RED);
                    usr_print(">>> ");
                } else {
                    usr_set_color(WHITE);
                    usr_print("    ");
                }
                match i {
                    0 => usr_set_color(YELLOW),
                    1 => usr_set_color(BLUE),
                    2 => usr_set_color(HHU_GREEN),
                    _ => {}
                }
                usr_print("[");
                print_usize(i);
                usr_print("] ");
                usr_set_color(WHITE);
                print_u32_pad5(counters[i]);
                usr_print("   ");
            }

            usr_set_text_cursor(0, counter_start_y + 16);
            usr_set_color(HHU_BLUE);
            usr_print("Currently running: Thread ");
            print_usize(thread_index);

            usr_set_text_cursor(0, counter_start_y + 32);
            usr_set_color(YELLOW);
            usr_print("Recent pattern: ");
            if switch_len >= 10 {
                let start = switch_len - 10;
                for j in start..switch_len {
                    print_usize(switches[j]);
                    if j < switch_len - 1 {
                        usr_print("->");
                    }
                }
            }
        }

        let total: u32 = counters.iter().sum();
        if total >= 15_000 {
            active = false;
            usr_set_text_cursor(0, counter_start_y + 48);
            usr_set_color(RED);
            usr_print("=== Demo Complete ===\n");
            usr_set_color(WHITE);
            usr_set_color(HHU_BLUE);
            usr_print("yozora$ ");
            usr_set_color(WHITE);
        }

        usr_thread_yield();
    }

    usr_thread_exit();
}

fn print_usize(v: usize) {
    let mut buf = [0u8; 20];
    let n = write_usize(v, &mut buf);
    if let Ok(t) = core::str::from_utf8(&buf[..n]) {
        usr_print(t);
    }
}

fn print_u32_pad5(v: u32) {
    let mut d = [b'0'; 5];
    let mut n = v;
    for i in (0..5).rev() {
        d[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    usr_print(unsafe { core::str::from_utf8_unchecked(&d) });
}

fn write_usize(mut v: usize, buf: &mut [u8]) -> usize {
    if v == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut i = 0;
    while v > 0 {
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        i += 1;
    }
    for j in 0..i / 2 {
        buf.swap(j, i - 1 - j);
    }
    i
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
