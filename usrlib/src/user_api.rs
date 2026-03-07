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
