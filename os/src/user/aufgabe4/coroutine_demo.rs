use crate::devices::cga;
use crate::kernel::coroutines::coroutine::Coroutine;
use crate::devices::cga::CGA;
use crate::devices::cga::Color;
use crate::consts;
const POSITIONS: [(usize, usize); 3] = [(10, 5), (30, 5), (50, 5)];
fn coroutine_loop(coroutine: &mut Coroutine) {
    let id = coroutine.get_id();
    let (x, y) = POSITIONS[id % 3];
    let mut counter = 0;
    let attr_yellow = CGA.lock().attribute(Color::Black, Color::Yellow, false);
    let attr_green = CGA.lock().attribute(Color::Black, Color::Green, false);

    loop {
        {
            let mut cga = CGA.lock();
            cga.setpos(x, y);
            cga.show(x, y, '[', attr_yellow);
            cga.show(x + 1, y, char::from_digit(id as u32, 10).unwrap_or('?'), attr_yellow);
            cga.show(x + 2, y, ']', attr_yellow);
            cga.show(x + 4, y, char::from_digit((counter / 100) % 10, 10).unwrap_or(' '), attr_green);
            cga.show(x + 5, y, char::from_digit((counter / 10) % 10, 10).unwrap_or(' '), attr_green);
            cga.show(x + 6, y, char::from_digit(counter % 10, 10).unwrap_or(' '), attr_green);
        }
        counter += 1;
        coroutine.switch();
    }
    kprintln!("Coroutine {} finished", id);
    /* Hier muss Code eingefuegt werden */

}

pub fn run() {
    kprintln!("Creating coroutines");
    let mut c1 = Coroutine::new(coroutine_loop);
    let mut c2 = Coroutine::new(coroutine_loop);
    let mut c3 = Coroutine::new(coroutine_loop);

    kprintln!("Setting up coroutine chain");
    c1.set_next(&mut c2);
    c2.set_next(&mut c3);
    c3.set_next(&mut c1);

    kprintln!("Starting coroutines");
    c1.start();
    kprintln!("This should never be reached");
    /* Hier muss Code eingefuegt werden */

}