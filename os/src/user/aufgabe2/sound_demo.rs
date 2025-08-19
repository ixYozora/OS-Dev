use crate::devices::pcspk;
use crate::devices::cga;
use crate::devices::cga_print;
use crate::devices::cga::CGA;
use crate::devices::cga::Color;

pub fn run() {
 
   /* Hier muss Code eingefuegt werden */
    println!("Sound Demo Tetris");
    pcspk::tetris();
    println!("Sound Demo aerodynamics");
    pcspk::aerodynamic();
}
