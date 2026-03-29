use crate::devices::lfb;

pub extern "C" fn sys_fb_get_dims() -> u64 {
    if !lfb::is_lfb_initialized() {
        return 0;
    }
    let guard = lfb::get_lfb().lock();
    let (w, h) = guard.get_dimensions();
    ((w as u64) << 32) | (h as u64)
}

pub extern "C" fn sys_fb_draw_pixel(x: u64, y: u64, color: u64) -> u64 {
    if !lfb::is_lfb_initialized() {
        return 0;
    }
    let mut guard = lfb::get_lfb().lock();
    guard.draw_pixel(x as u32, y as u32, color as u32);
    0
}

pub extern "C" fn sys_fb_draw_bitmap(
    x: u64,
    y: u64,
    w: u64,
    h: u64,
    ptr: *const u8,
    len: u64,
) -> u64 {
    if !lfb::is_lfb_initialized() || ptr.is_null() {
        return 0;
    }
    let w = w as u32;
    let h = h as u32;
    let need = (w as u64).saturating_mul(h as u64).saturating_mul(3);
    if len < need {
        return 0;
    }
    let slice = unsafe { core::slice::from_raw_parts(ptr, len as usize) };
    let mut guard = lfb::get_lfb().lock();
    guard.draw_bitmap(x as u32, y as u32, w, h, slice);
    0
}
