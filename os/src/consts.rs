// Stack size for each new thread
pub const STACK_SIZE: usize = 0x80000;             // 512 KB for each stack
pub const STACK_ALIGNMENT: usize = 8; 
pub const STACK_ENTRY_SIZE: usize = 8;

pub const HEAP_SIZE: usize  = 32 * 1024 * 1024;    // 32 MB heap size

pub const PAGE_FRAME_SIZE: usize = 4096;            // 4 KiB page frames

/// Size of a virtual page (4 KiB)
pub const PAGE_SIZE: usize = 0x1000;

/// Start address of the user stack in virtual memory (64 TiB)
pub const USER_STACK_VIRT_START:usize = 0x4000_0000_0000;
/// End address of the user stack in virtual memory
pub const USER_STACK_VIRT_END: usize = USER_STACK_VIRT_START + STACK_SIZE;