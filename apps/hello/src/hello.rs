#![no_std]

use core::panic::PanicInfo;
use usrlib::user_api::{usr_hello_world, usr_get_process_id, usr_print, usr_thread_exit};

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_hello_world();

    let pid = usr_get_process_id();
    let mut buf = [0u8; 32];
    let msg = format_pid(pid, &mut buf);
    usr_print(msg);

    usr_thread_exit();
}

fn format_pid(pid: usize, buf: &mut [u8]) -> &str {
    let prefix = b"Process ID: ";
    let prefix_len = prefix.len();
    buf[..prefix_len].copy_from_slice(prefix);

    if pid == 0 {
        buf[prefix_len] = b'0';
        buf[prefix_len + 1] = b'\n';
        unsafe { core::str::from_utf8_unchecked(&buf[..prefix_len + 2]) }
    } else {
        let mut num = pid;
        let mut digits = [0u8; 20];
        let mut len = 0;
        while num > 0 {
            digits[len] = b'0' + (num % 10) as u8;
            num /= 10;
            len += 1;
        }
        for i in 0..len {
            buf[prefix_len + i] = digits[len - 1 - i];
        }
        buf[prefix_len + len] = b'\n';
        unsafe { core::str::from_utf8_unchecked(&buf[..prefix_len + len + 1]) }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
