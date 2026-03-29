#![no_std]

extern crate alloc;

mod bmp_hhu;

use core::panic::PanicInfo;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{
    usr_fb_draw_bitmap, usr_fb_draw_pixel, usr_fb_get_dims, usr_spawn_process,     usr_thread_exit,
};

#[global_allocator]
static _A: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    let _ = usr_spawn_process("sound_tetris");

    let packed = usr_fb_get_dims();
    if packed == 0 {
        usr_thread_exit();
    }
    let w = (packed >> 32) as u32;
    let h = (packed & 0xffff_ffff) as u32;

    for y in 0..h {
        for x in 0..w {
            let color = linear_interpolate_2d(
                x,
                y,
                w,
                h,
                0x0000ff,
                0x00ff00,
                0xff0000,
                0xffff00,
            );
            usr_fb_draw_pixel(x, y, color);
        }
    }

    let bmp_x = (w.saturating_sub(bmp_hhu::WIDTH)) / 2;
    let bmp_y = (h.saturating_sub(bmp_hhu::HEIGHT)) / 2;
    usr_fb_draw_bitmap(
        bmp_x,
        bmp_y,
        bmp_hhu::WIDTH,
        bmp_hhu::HEIGHT,
        bmp_hhu::DATA,
    );

    usr_thread_exit();
}

fn linear_interpolate_1d(x: u32, xr: u32, l: u32, r: u32) -> u32 {
    ((((l >> 16) * (xr - x) + (r >> 16) * x) / xr) << 16)
        | (((((l >> 8) & 0xff) * (xr - x) + ((r >> 8) & 0xff) * x) / xr) << 8)
        | (((l & 0xff) * (xr - x) + (r & 0xff) * x) / xr)
}

fn linear_interpolate_2d(
    x: u32,
    y: u32,
    xres: u32,
    yres: u32,
    lt: u32,
    rt: u32,
    lb: u32,
    rb: u32,
) -> u32 {
    linear_interpolate_1d(
        y,
        yres,
        linear_interpolate_1d(x, xres, lt, rt),
        linear_interpolate_1d(x, xres, lb, rb),
    )
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
