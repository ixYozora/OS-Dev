use crate::devices::{cga, pit};
use crate::devices::pcspk::tetris;
use crate::kernel::threads::scheduler;
use crate::kernel::threads::scheduler::{get_scheduler, Scheduler};
use crate::kernel::threads::thread::Thread;
use crate::user::aufgabe4::hello_world_thread::hello_world;

fn thread_entry() {

    let id = get_scheduler().get_active_tid();
    let mut counter = 0;
    loop {
        if let Some(mut cga) = cga::CGA.try_lock() {
            cga.setpos(10, 10 + id);
            print_cga!(&mut cga, "Thread [{}]: {}", id, counter);
        }


        //cga.setpos(10, 10 + id);
        //print_cga!(&mut cga, "Thread [{}]: {}", id, counter);


        //cga::CGA.lock().setpos(10, 10 + id);
        //println!("Thread [{}]: {}", id, counter);
        //pit::wait(100);

        counter += 1;

        if counter % 20 == 0 {
            get_scheduler().yield_cpu();
        }
    }

}

fn play_tetris() {
    tetris();
}

pub fn run() {

    cga::CGA.lock().clear();
    println!("Starting thread demo...");

    let mut scheduler = get_scheduler();

    let thread1 = Thread::new(thread_entry);
    let thread2 = Thread::new(thread_entry);
    let thread3 = Thread::new(thread_entry);

    let tetris_thread = Thread::new(play_tetris);

    scheduler.ready(thread1);
    scheduler.ready(thread2);
    scheduler.ready(thread3);
    scheduler.ready(tetris_thread);

    scheduler.schedule();

}