/*
 * Module: user_api
 *
 * Description: All system calls available to user programs are defined in this module.
 *
 * Author: Stefan Lankes, RWTH Aachen University
 *         Licensed under the Apache License, Version 2.0 or MIT license, at your option.
 *
 *         Michael Schoettner, Heinrich Heine University Duesseldorf, 14.09.2023
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 15.10.2025
 */

use core::arch::asm;

/// System call numbers available to user programs.
/// Order must match the SYSCALL_TABLE in syscall_dispatcher.rs.
#[repr(u64)]
pub enum SyscallFunction {
    HelloWorld,
    ThreadYield,
    ThreadExit,
    ThreadGetId,
    GetSystemTime,
    Print,
    GetChar,
    GetProcessId,
    DumpVmas,
    MapHeap,
    SpawnProcess,
    WaitPid,
    SetColor,
    BuffClear,
    GetKey,
    PcspkPlayTune,
    FbGetDims,
    FbDrawPixel,
    FbDrawBitmap,
    GetTextCursor,
    SetTextCursor,
    ClearTextBands,
    NumSyscalls,
}

// ---------------------------------------------------------------------------
// User-facing syscall wrappers
// ---------------------------------------------------------------------------

pub fn usr_hello_world() {
    syscall0(SyscallFunction::HelloWorld);
}

pub fn usr_thread_yield() {
    syscall0(SyscallFunction::ThreadYield);
}

pub fn usr_thread_exit() {
    syscall0(SyscallFunction::ThreadExit);
}

pub fn usr_thread_get_id() -> usize {
    syscall0(SyscallFunction::ThreadGetId) as usize
}

pub fn usr_get_system_time() -> usize {
    syscall0(SyscallFunction::GetSystemTime) as usize
}

pub fn usr_print(msg: &str) {
    syscall2(SyscallFunction::Print, msg.as_ptr() as u64, msg.len() as u64);
}

pub fn usr_get_char() -> char {
    let c = syscall0(SyscallFunction::GetChar) as u8;
    c as char
}

pub fn usr_get_process_id() -> usize {
    syscall0(SyscallFunction::GetProcessId) as usize
}

pub fn usr_dump_vmas() {
    syscall0(SyscallFunction::DumpVmas);
}

pub fn usr_map_heap(user_heap_start: u64, user_heap_size: usize) {
    syscall2(SyscallFunction::MapHeap, user_heap_start, user_heap_size as u64);
}

pub fn usr_spawn_process(name: &str) -> usize {
    syscall2(SyscallFunction::SpawnProcess, name.as_ptr() as u64, name.len() as u64) as usize
}

pub fn usr_wait_pid(pid: usize) {
    syscall1(SyscallFunction::WaitPid, pid as u64);
}

pub fn usr_set_color(color: u32) {
    syscall1(SyscallFunction::SetColor, color as u64);
}

pub fn usr_buff_clear() {
    syscall0(SyscallFunction::BuffClear);
}

/// Returns a packed key value: (scancode << 8) | ascii.
/// scancode = 0 means no valid key was available.
pub fn usr_get_key() -> u64 {
    syscall0(SyscallFunction::GetKey)
}

/// PC speaker tunes from kernel (Ring 0 only in hardware). `tune_id`: 0 = tetris, 1 = aerodynamic, 2 = both.
pub fn usr_pcspk_play(tune_id: u64) {
    syscall1(SyscallFunction::PcspkPlayTune, tune_id);
}

/// Returns `(width << 32) | height`, or 0 if no linear framebuffer.
pub fn usr_fb_get_dims() -> u64 {
    syscall0(SyscallFunction::FbGetDims)
}

pub fn usr_fb_draw_pixel(x: u32, y: u32, color: u32) {
    syscall3(
        SyscallFunction::FbDrawPixel,
        x as u64,
        y as u64,
        color as u64,
    );
}

pub fn usr_fb_draw_bitmap(x: u32, y: u32, w: u32, h: u32, rgb: &[u8]) {
    syscall6(
        SyscallFunction::FbDrawBitmap,
        x as u64,
        y as u64,
        w as u64,
        h as u64,
        rgb.as_ptr() as u64,
        rgb.len() as u64,
    );
}

/// `(x << 32) | y` in pixels; 0 if no LFB.
pub fn usr_get_text_cursor() -> u64 {
    syscall0(SyscallFunction::GetTextCursor)
}

pub fn usr_set_text_cursor(x: u32, y: u32) {
    syscall2(SyscallFunction::SetTextCursor, x as u64, y as u64);
}

/// Clear `count` text rows starting at `base_y`, spaced by `step_px` (use 16 to match legacy shell demos).
pub fn usr_clear_text_bands(base_y: u32, step_px: u32, count: u32) {
    syscall3(
        SyscallFunction::ClearTextBands,
        base_y as u64,
        step_px as u64,
        count as u64,
    );
}

// ---------------------------------------------------------------------------
// Low-level syscall invocation (System V AMD64 ABI register convention)
// ---------------------------------------------------------------------------

#[inline(always)]
pub fn syscall0(syscall: SyscallFunction) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") syscall as u64 => ret,
            options(preserves_flags, nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall1(syscall: SyscallFunction, arg1: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") syscall as u64 => ret,
            in("rdi") arg1,
            options(preserves_flags, nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall2(syscall: SyscallFunction, arg1: u64, arg2: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") syscall as u64 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            options(preserves_flags, nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall3(syscall: SyscallFunction, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") syscall as u64 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            options(preserves_flags, nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall6(
    syscall: SyscallFunction,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") syscall as u64 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("rcx") arg4,
            in("r8") arg5,
            in("r9") arg6,
            options(preserves_flags, nostack)
        );
    }
    ret
}

// ===========================================================================
// Fast syscall wrappers (via syscall/sysret instruction)
// ===========================================================================

// Note: The syscall instruction clobbers RCX (saves RIP) and R11 (saves RFLAGS).
// Therefore the 4th parameter uses R10 instead of RCX.

pub fn fast_usr_hello_world() {
    fast_syscall0(SyscallFunction::HelloWorld);
}

pub fn fast_usr_thread_yield() {
    fast_syscall0(SyscallFunction::ThreadYield);
}

pub fn fast_usr_thread_exit() {
    fast_syscall0(SyscallFunction::ThreadExit);
}

pub fn fast_usr_thread_get_id() -> usize {
    fast_syscall0(SyscallFunction::ThreadGetId) as usize
}

pub fn fast_usr_get_system_time() -> usize {
    fast_syscall0(SyscallFunction::GetSystemTime) as usize
}

pub fn fast_usr_print(msg: &str) {
    fast_syscall2(SyscallFunction::Print, msg.as_ptr() as u64, msg.len() as u64);
}

pub fn fast_usr_get_char() -> char {
    let c = fast_syscall0(SyscallFunction::GetChar) as u8;
    c as char
}

pub fn fast_usr_get_process_id() -> usize {
    fast_syscall0(SyscallFunction::GetProcessId) as usize
}

pub fn fast_usr_dump_vmas() {
    fast_syscall0(SyscallFunction::DumpVmas);
}

pub fn fast_usr_map_heap(user_heap_start: u64, user_heap_size: usize) {
    fast_syscall2(SyscallFunction::MapHeap, user_heap_start, user_heap_size as u64);
}

pub fn fast_usr_spawn_process(name: &str) -> usize {
    fast_syscall2(SyscallFunction::SpawnProcess, name.as_ptr() as u64, name.len() as u64) as usize
}

pub fn fast_usr_wait_pid(pid: usize) {
    fast_syscall1(SyscallFunction::WaitPid, pid as u64);
}

pub fn fast_usr_set_color(color: u32) {
    fast_syscall1(SyscallFunction::SetColor, color as u64);
}

pub fn fast_usr_buff_clear() {
    fast_syscall0(SyscallFunction::BuffClear);
}

pub fn fast_usr_get_key() -> u64 {
    fast_syscall0(SyscallFunction::GetKey)
}

pub fn fast_usr_pcspk_play(tune_id: u64) {
    fast_syscall1(SyscallFunction::PcspkPlayTune, tune_id);
}

pub fn fast_usr_fb_get_dims() -> u64 {
    fast_syscall0(SyscallFunction::FbGetDims)
}

pub fn fast_usr_fb_draw_pixel(x: u32, y: u32, color: u32) {
    fast_syscall3(
        SyscallFunction::FbDrawPixel,
        x as u64,
        y as u64,
        color as u64,
    );
}

pub fn fast_usr_fb_draw_bitmap(x: u32, y: u32, w: u32, h: u32, rgb: &[u8]) {
    fast_syscall6(
        SyscallFunction::FbDrawBitmap,
        x as u64,
        y as u64,
        w as u64,
        h as u64,
        rgb.as_ptr() as u64,
        rgb.len() as u64,
    );
}

pub fn fast_usr_get_text_cursor() -> u64 {
    fast_syscall0(SyscallFunction::GetTextCursor)
}

pub fn fast_usr_set_text_cursor(x: u32, y: u32) {
    fast_syscall2(SyscallFunction::SetTextCursor, x as u64, y as u64);
}

pub fn fast_usr_clear_text_bands(base_y: u32, step_px: u32, count: u32) {
    fast_syscall3(
        SyscallFunction::ClearTextBands,
        base_y as u64,
        step_px as u64,
        count as u64,
    );
}

#[inline(always)]
pub fn fast_syscall0(syscall: SyscallFunction) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") syscall as u64 => ret,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn fast_syscall1(syscall: SyscallFunction, arg1: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") syscall as u64 => ret,
            in("rdi") arg1,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn fast_syscall2(syscall: SyscallFunction, arg1: u64, arg2: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") syscall as u64 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn fast_syscall3(syscall: SyscallFunction, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") syscall as u64 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn fast_syscall6(
    syscall: SyscallFunction,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            inlateout("rax") syscall as u64 => ret,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            in("r8") arg5,
            in("r9") arg6,
            out("rcx") _,
            out("r11") _,
            options(nostack)
        );
    }
    ret
}
