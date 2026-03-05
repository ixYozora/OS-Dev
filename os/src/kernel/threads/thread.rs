/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: thread                                                          ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Functions for creating, starting, switching and ending threads. ║
   ║         Supports both kernel threads (Ring 0) and user threads (Ring 3).║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Autor:  Michael Schoettner, 15.05.2023                                  ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::arch::naked_asm;
use core::fmt::Display;
use core::sync::atomic::AtomicUsize;
use core::{fmt, ptr};

use crate::consts;
use crate::consts::{STACK_SIZE, USER_STACK_VIRT_START, USER_STACK_VIRT_END};
use crate::devices::pit;
use crate::kernel::cpu;
use crate::kernel::paging::pages;
use crate::kernel::syscalls::user_api::usr_thread_exit;
use crate::kernel::threads::scheduler::get_scheduler;

unsafe extern "C" {
    fn _tss_set_rsp0(rsp0: usize);
}

static THREAD_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn next_id() -> usize {
    THREAD_ID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst)
}

#[naked]
unsafe extern "C" fn thread_start(stack_ptr: usize) {
    unsafe {
        naked_asm!(
            "mov rsp, rdi",
            "call unlock_scheduler",
            "xor rbp, rbp",
            "popf",
            "pop rbp",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "ret",
        )
    }
}

/// Context switch: save current context, restore next, switch address space, update TSS rsp0.
/// Parameters: rdi = &current.stack_ptr, rsi = next.stack_ptr,
///             rdx = next kernel stack end, rcx = next PML4 physical address
#[naked]
unsafe extern "C" fn thread_switch(
    current_stack_ptr: *mut usize,
    next_stack: usize,
    next_stack_end: usize,
    next_pml4: u64,
) {
    unsafe {
        naked_asm!(
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            "push rax",
            "push rbx",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push rbp",
            "pushf",
            "mov [rdi], rsp",
            "mov rsp, rsi",
            "mov cr3, rcx",
            "mov rdi, rdx",
            "call _tss_set_rsp0",
            "call unlock_scheduler",
            "popf",
            "pop rbp",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "ret",
        )
    }
}

/// Switch to Ring 3 via iretq. Builds the interrupt return frame and executes iretq.
/// rdi = entry point (RIP), rsi = user stack pointer (RSP)
#[naked]
unsafe extern "C" fn thread_user_start(entry: usize, user_stack: usize) {
    unsafe {
        naked_asm!(
            "push 0x23",                    // SS: user data selector 0x20 | RPL 3
            "push rsi",                      // RSP: user stack pointer
            "pushf",                         // RFLAGS
            "or qword ptr [rsp], 0x200",     // set IF (bit 9) for user mode
            "push 0x2B",                    // CS: user code selector 0x28 | RPL 3
            "push rdi",                      // RIP: entry function
            "iretq",
        )
    }
}

#[repr(C)]
pub struct Thread {
    id: usize,
    kernel_stack: Vec<u64>,
    user_stack: Vec<u64>,
    stack_ptr: usize,
    entry: fn(),
    is_kernel_thread: bool,
    pml4_addr: u64,
}

fn allocate_stack() -> Vec<u64> {
    let mut stack = Vec::<u64>::with_capacity(STACK_SIZE / 8);
    for _ in 0..stack.capacity() {
        stack.push(0);
    }
    stack
}

impl Thread {
    pub fn new_kernel_thread(entry: fn()) -> Box<Thread> {
        let pml4 = pages::init_kernel_tables();
        let pml4_addr = ptr::from_ref(pml4) as u64;

        let kernel_stack = allocate_stack();
        let user_stack = allocate_stack();
        let stack_ptr = ptr::from_ref(&kernel_stack[kernel_stack.capacity() - 1]) as usize;

        let mut thread = Box::new(Thread {
            id: next_id(),
            kernel_stack,
            user_stack,
            stack_ptr,
            entry,
            is_kernel_thread: true,
            pml4_addr,
        });

        thread.prepare_kernel_stack(Thread::kickoff_kernel_thread as u64);
        thread
    }

    pub fn new_user_thread(entry: fn()) -> Box<Thread> {
        let pml4 = pages::init_kernel_tables();
        let pml4_addr = ptr::from_ref(pml4) as u64;

        // Map user stack at the fixed virtual address in this thread's address space
        unsafe { pages::map_user_stack(pml4); }

        // Wrap the virtual address range as a Vec (no actual heap allocation)
        let user_stack = unsafe {
            Vec::from_raw_parts(
                USER_STACK_VIRT_START as *mut u64,
                STACK_SIZE / 8,
                STACK_SIZE / 8,
            )
        };

        let kernel_stack = allocate_stack();
        let stack_ptr = ptr::from_ref(&kernel_stack[kernel_stack.capacity() - 1]) as usize;

        let mut thread = Box::new(Thread {
            id: next_id(),
            kernel_stack,
            user_stack,
            stack_ptr,
            entry,
            is_kernel_thread: false,
            pml4_addr,
        });

        thread.prepare_kernel_stack(Thread::kickoff_user_thread as u64);
        thread
    }

    pub fn start(&mut self) {
        unsafe {
            pages::write_cr3_raw(self.pml4_addr);
            thread_start(self.stack_ptr);
        }
    }

    pub unsafe fn switch(current: *mut Thread, next: *mut Thread) {
        unsafe {
            if next.is_null() {
                panic!("No Thread!");
            }
            let next_stack_end = (*next).get_kernel_stack_end();
            let next_pml4 = (*next).pml4_addr;
            thread_switch(
                &mut (*current).stack_ptr,
                (*next).stack_ptr,
                next_stack_end,
                next_pml4,
            );
        }
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    fn get_kernel_stack_end(&self) -> usize {
        let cap = self.kernel_stack.capacity();
        ptr::from_ref(&self.kernel_stack[cap - 1]) as usize + 8
    }

    fn get_user_stack_end(&self) -> usize {
        if self.is_kernel_thread {
            let cap = self.user_stack.capacity();
            ptr::from_ref(&self.user_stack[cap - 1]) as usize + 8
        } else {
            USER_STACK_VIRT_END
        }
    }

    fn prepare_kernel_stack(&mut self, kickoff: u64) {
        let thread = ptr::from_mut(self) as u64;
        let length = self.kernel_stack.len();

        self.kernel_stack[length - 1] = 0x131155;
        self.kernel_stack[length - 2] = kickoff;
        self.kernel_stack[length - 3] = 0; // r8
        self.kernel_stack[length - 4] = 0; // r9
        self.kernel_stack[length - 5] = 0; // r10
        self.kernel_stack[length - 6] = 0; // r11
        self.kernel_stack[length - 7] = 0; // r12
        self.kernel_stack[length - 8] = 0; // r13
        self.kernel_stack[length - 9] = 0; // r14
        self.kernel_stack[length - 10] = 0; // r15
        self.kernel_stack[length - 11] = 0; // rax
        self.kernel_stack[length - 12] = 0; // rbx
        self.kernel_stack[length - 13] = 0; // rcx
        self.kernel_stack[length - 14] = 0; // rdx
        self.kernel_stack[length - 15] = 0; // rsi
        self.kernel_stack[length - 16] = thread; // rdi -> first parameter for kickoff
        self.kernel_stack[length - 17] = 0; // rbp
        self.kernel_stack[length - 18] = 0x2; // rflags (IE = 0); interrupts disabled

        self.stack_ptr = self.stack_ptr - (consts::STACK_ENTRY_SIZE * 17);
    }

    fn kickoff_kernel_thread(&self) {
        let stack_end = self.get_kernel_stack_end();
        unsafe {
            _tss_set_rsp0(stack_end);
        }
        cpu::enable_int();
        (self.entry)();
        get_scheduler().exit();
    }

    fn kickoff_user_thread(&self) {
        let stack_end = self.get_kernel_stack_end();
        unsafe {
            _tss_set_rsp0(stack_end);
        }
        let user_stack = self.get_user_stack_end();

        let user_sp = user_stack - 8;
        unsafe {
            *(user_sp as *mut usize) = user_thread_exit_trampoline as usize;
            thread_user_start(self.entry as usize, user_sp);
        }
    }
}

fn user_thread_exit_trampoline() {
    usr_thread_exit();
}

pub fn sleep_ms(ms: usize) {
    pit::wait(ms);
}

impl Display for Thread {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "T{}", self.id)
    }
}
