// Stack size for each new thread
pub const STACK_SIZE: usize = 0x80000;             // 512 KB for each stack
pub const STACK_ALIGNMENT: usize = 8; 
pub const STACK_ENTRY_SIZE: usize = 8;

pub const HEAP_SIZE: usize  = 32 * 1024 * 1024;    // 32 MB heap size

pub const PAGE_FRAME_SIZE: usize = 4096;            // 4 KiB page frames
