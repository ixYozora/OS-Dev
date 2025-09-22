# A tiny x86_64 OS in Rust (graphics, preemptive threads, shell)

## Overview
A teaching/learning OS kernel for x86_64 written in Rust.

- Preemptive round‑robin threads (PIT 1 kHz) with cooperative yield support
- Interrupt pipeline: PIC remap, IDT setup, Rust interrupt dispatcher
- Hand‑written context switch in assembly (naked functions)
- Custom concurrency primitives: Spinlock and Mutex with waiter queue
- Linear Framebuffer (graphics) with text rendering (8×8 font)
- Graphics shell with history/line editing and async demo launcher
- PC speaker driver (PIT channel 2) playing simple tunes
- Heap allocator (linked‑list/free‑list) with demo and free‑list dump
- Minimal PCI bus scan example

## Demos
- `text`: formatted number output
- `keyboard`: live key echo (Enter/Backspace/ESC)
- `heap`: Box allocation/free + free‑list dump
- `sound`: PC speaker tunes (`tetris`, `aerodynamic`)
- `graphics`: simple LFB drawing
- `threads`: 3 counters + kill/exit flow
- `synchronize`: lock competition (Mutex vs Spinlock visualization)

## Architecture (big picture)
- Boot and init:
  - PIC and IDT loaded; keyboard and PIT plug in
  - Multiboot framebuffer initialized (e.g., 800×600×32)
  - Scheduler created with idle + shell thread; `schedule()` starts them
- Interrupt pipeline:
  - Device → PIC (0x20–0x2F) → IDT → ASM stub → Rust dispatcher → registered ISR
  - TimerISR: increments system time, tries to draw a spinner (`try_lock`), force‑unlocks dispatcher lock, then opportunistically calls `scheduler.yield_cpu()`

## Scheduling and threads
- Round‑robin (no priorities): one active thread, others in a FIFO `ready_queue`
- Cooperative yield: `yield_cpu()` moves active → end of queue
- Preemption: PIT ISR triggers yield when safe (scheduler lock `try_lock` succeeds and allocator not locked)
- Context switch (ASM):
  - Save regs/flags on old stack; store old `rsp`
  - Load next `rsp`; immediately call `unlock_scheduler` (Rust scope won’t drop the lock)
  - Restore regs/flags; `ret` to next thread (continuation or first‑time kickoff)

## Concurrency (Spinlock & Mutex)
- Spinlock
  - Busy‑wait on an `AtomicBool` (`swap(true)`); very short critical sections
  - Used where blocking is not allowed (e.g., internal wait queues, interrupt dispatcher)
- Mutex with waiter queue
  - `lock()`: if busy, `prepare_block()` current thread, enqueue to `mutex.wait_queue`, `switch_from_blocked_thread()`
  - `unlock()`: `store(false)` and `ready_after_block()` exactly one waiter (no immediate switch)
  - Fair (FIFO), efficient under contention (no busy‑wait)
- ISR rule: never block; use `try_lock` only

## Graphics output (LFB & Writer)
- LFB (linear framebuffer) 32bpp ARGB
- Writer (global, `Mutex`‑protected) handles color, cursor, newline, backspace, scroll
- `buff_print!`:
  - Locks Writer → locks LFB once per string → renders → unlocks (atomic, flicker‑free)
- Spinner (top‑right heartbeat):
  - Drawn from TimerISR every 250 ms only if `try_lock` succeeds and scheduler/allocator are free

## PC speaker
- Tone: PIT channel 2 (0x42), Mode 3 (0xB6), divisor = `1193180 / freq`
- On/off via PPI 0x61 bits
- Duration via `pit::wait(ms)` using `SYSTEM_TIME` (do not touch PIT channel 0)

## Heap allocator
- Linked‑list (free‑list) allocator
  - Splits blocks on allocation; dumps free list for debugging
  - Note: provided code does not coalesce adjacent free blocks on free
- Heap demo shows before/after states and object address

## Build and run
Requirements:
- Rust nightly, `cargo`, `cargo-make`
- `qemu-system-x86_64`
- `nasm` (for boot code)

Build:
- Development: `cargo make qemu`
- Optimized (faster graphics): `cargo make --profile production qemu`

QEMU audio (PC speaker):
- Newer QEMU: `-audiodev pa,id=snd0 -machine pcspk-audiodev=snd0`
- Some builds: `-soundhw pcspk`

Graphics vs text mode:
- In `boot/boot.asm`, comment out `TEXT_MODE` to boot graphics (e.g., 800×600×32)
- Adjust `MULTIBOOT_GRAPHICS_*` if needed (VESA‑like modes)

## Repository layout (key parts)
- `boot/`: bootloader bits (Multiboot), graphics toggles
- `startup.rs`: init devices, LFB, spawn shell, start scheduler
- `kernel/`
  - `interrupts/`: IDT, PIC, Rust dispatcher (`INT_VECTORS`)
  - `threads/`: `thread`, `scheduler` (ready queue), ASM switching
  - `coroutines/`: coroutine task (educational stepping stone)
  - `allocator/`: bump and linked‑list allocators
- `devices/`
  - `keyboard`, `cga` (text), `lfb` (graphics), `pcspk` (speaker), `pci`, `kprint`
  - `buff_print/`: Writer‑based text output over LFB
- `library/`
  - `queue` (linked queue), `spinlock`, `mutex` (with waiter queue)
- `user/`: demos

## Safety & locking rules
- In ISRs:
  - Minimal work, no blocking, only `try_lock`
  - TimerISR: force‑unlock `INT_VECTORS` before thread switch (you don’t return normally)
- During thread switch (ASM):
  - Immediately call `unlock_scheduler` after stack switch
- Opportunistic preemption:
  - Switch only if scheduler lock is free (`try_lock`) and allocator not locked
- Fixed lock order for output: Writer → LFB
- Allocator:
  - Scheduler checks `allocator::is_locked()` before preemptive switch

## Known limitations
- Linked‑list allocator does not coalesce free blocks (fragmentation can grow)
- Single‑core, APIC/SMP not implemented
- Educational design with conservative ISR policies

## Credits
- Allocator design inspired by Philipp Oppermann (https://os.phil-opp.com/allocator-designs/)
- Course templates and tasks by Michael Schoettner (HHU)
