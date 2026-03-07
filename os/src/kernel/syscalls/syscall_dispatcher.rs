/*
 * Module: syscall_dispatcher
 *
 * Description: All system calls are routed here via the IDT syscall handler (interrupt 0x80)
 *              or the fast syscall handler (syscall instruction).
 *              The system call number is passed in the rax register and is used as index into
 *              the syscall table to call the corresponding function.
 */

use core::arch::{asm, naked_asm};
use crate::kernel::syscalls::functions::hello::sys_hello_world;
use crate::kernel::syscalls::functions::threads::{sys_thread_yield, sys_thread_exit, sys_thread_get_id, sys_get_process_id};
use crate::kernel::syscalls::functions::io::{sys_get_system_time, sys_print, sys_get_char};
use usrlib::user_api::SyscallFunction;

unsafe extern "C" {
    static _kernel_rsp0: u64;
}

/// Global syscall function table.
static SYSCALL_TABLE: SyscallFunctionTable = SyscallFunctionTable::new();

/// Struct to hold the syscall function pointers.
#[repr(align(64))]
#[repr(C)]
struct SyscallFunctionTable {
    table: [*const u64; SyscallFunction::NumSyscalls as usize],
}

impl SyscallFunctionTable {
    pub const fn new() -> SyscallFunctionTable {
        SyscallFunctionTable {
            table: [
                sys_hello_world as *const u64,
                sys_thread_yield as *const u64,
                sys_thread_exit as *const u64,
                sys_thread_get_id as *const u64,
                sys_get_system_time as *const u64,
                sys_print as *const u64,
                sys_get_char as *const u64,
                sys_get_process_id as *const u64,
            ],
        }
    }
}

unsafe impl Send for SyscallFunctionTable {}
unsafe impl Sync for SyscallFunctionTable {}

// ---------------------------------------------------------------------------
// Interrupt-based syscall dispatcher (int 0x80)
// ---------------------------------------------------------------------------

/// System call dispatcher (interrupt 0x80).
/// Syscall number in rax, parameters in rdi/rsi/rdx/rcx/r8/r9 per System V ABI.
/// Return value passed back in rax.
#[unsafe(naked)]
pub extern "C" fn syscall_disp() {
    naked_asm!(
        // Save all registers except rax (syscall number / return value)
        "push rcx",
        "push rdx",
        "push rbx",
        "push rsi",
        "push rdi",
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // Validate syscall number and dispatch
        "cmp rax, {NUM_SYSCALLS}",
        "jge syscall_abort",
        "call [{SYSCALL_TABLE} + rax * 8]",

        // Restore all registers except rax
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rbx",
        "pop rdx",
        "pop rcx",

        "iretq",

        NUM_SYSCALLS = const SyscallFunction::NumSyscalls as usize,
        SYSCALL_TABLE = sym SYSCALL_TABLE
    )
}

// ---------------------------------------------------------------------------
// Fast syscall dispatcher (syscall/sysret)
// ---------------------------------------------------------------------------

/// Scratch space for user RSP during fast syscall entry.
static mut FAST_SYSCALL_USER_RSP: u64 = 0;

/// Fast system call handler, entered via the `syscall` instruction.
#[unsafe(naked)]
pub extern "C" fn fast_syscall_disp() {
    naked_asm!(
        // --- Switch to kernel stack (interrupts are disabled via FMASK) ---
        "mov [{user_rsp}], rsp",
        "mov rsp, [{kernel_rsp0}]",

        // Save user context that syscall stored in registers
        "push qword ptr [{user_rsp}]", // user RSP
        "push rcx",                     // user RIP
        "push r11",                     // user RFLAGS

        // Save all registers except rax
        "push rdx",
        "push rbx",
        "push rsi",
        "push rdi",
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // r10 carries the 4th parameter (rcx was clobbered by syscall instruction)
        "mov rcx, r10",

        // Re-enable interrupts so sys_* functions can use interrupt-driven I/O
        "sti",

        // Validate and dispatch
        "cmp rax, {NUM_SYSCALLS}",
        "jge syscall_abort",
        "call [{SYSCALL_TABLE} + rax * 8]",

        // Disable interrupts before restoring context
        "cli",

        // Restore all registers except rax
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rbx",
        "pop rdx",

        // Restore user context
        "pop r11",   // RFLAGS → restored by sysretq
        "pop rcx",   // RIP   → restored by sysretq
        "pop rsp",   // user RSP

        "sysretq",

        user_rsp = sym FAST_SYSCALL_USER_RSP,
        kernel_rsp0 = sym _kernel_rsp0,
        NUM_SYSCALLS = const SyscallFunction::NumSyscalls as usize,
        SYSCALL_TABLE = sym SYSCALL_TABLE,
    )
}

// ---------------------------------------------------------------------------
// MSR configuration for syscall/sysret
// ---------------------------------------------------------------------------

const IA32_EFER: u32  = 0xC000_0080;
const IA32_STAR: u32  = 0xC000_0081;
const IA32_LSTAR: u32 = 0xC000_0082;
const IA32_FMASK: u32 = 0xC000_0084;

unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe { asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nomem, nostack)); }
    (high as u64) << 32 | low as u64
}

unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe { asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nomem, nostack)); }
}

/// Initialize the MSRs required for fast syscall/sysret.
/// Must be called once during kernel startup after GDT and TSS are configured.
pub fn init_fast_syscalls() {
    unsafe {
        // Enable SCE (System Call Extensions) in IA32_EFER
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | 1); // bit 0 = SCE

        // IA32_STAR:
        //   [63:48] = SYSRET base selector  → 0x18 (so CS = 0x18+16 = 0x28, SS = 0x18+8 = 0x20)
        //   [47:32] = SYSCALL base selector → 0x10 (kernel CS = 0x10, kernel SS = 0x10+8 = 0x18)
        //   [31:0]  = reserved (target EIP for 32-bit SYSCALL, unused in long mode)
        let star: u64 = (0x0018u64 << 48) | (0x0010u64 << 32);
        wrmsr(IA32_STAR, star);

        // IA32_LSTAR = entry point for 64-bit SYSCALL
        wrmsr(IA32_LSTAR, fast_syscall_disp as u64);

        // IA32_FMASK = RFLAGS bits to clear on SYSCALL entry
        // Clear IF (bit 9) to disable interrupts, and DF (bit 10) for C ABI compliance
        wrmsr(IA32_FMASK, 0x600);
    }
}

// ---------------------------------------------------------------------------
// Shared abort handler
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
extern "C" fn syscall_abort() {
    panic!("Invalid syscall number");
}
