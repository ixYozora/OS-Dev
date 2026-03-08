#![no_std]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use usrlib::allocator::{Locked, LinkedListAllocator};
use usrlib::user_api::{
    usr_print, usr_get_char, usr_thread_exit,
    usr_get_system_time, usr_get_process_id, usr_map_heap,
    usr_spawn_process, usr_wait_pid,
    usr_set_color, usr_buff_clear, usr_get_key,
    usr_thread_get_id, usr_dump_vmas,
};

#[global_allocator]
static USER_ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new(0, 0));

const USER_HEAP_START: u64 = 0x200_0000_0000;
const USER_HEAP_SIZE: usize = 2 * 1024 * 1024;

const fn make_color(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

const WHITE: u32 = make_color(170, 170, 170);
const RED: u32 = make_color(170, 0, 0);
const HHU_BLUE: u32 = make_color(0, 106, 179);
const HHU_GREEN: u32 = make_color(151, 191, 13);

const PROMPT: &str = "yozora$ ";
const SHELL_BANNER: &str = r#"
 __    __
/\ \  /\ \
\ `\`\\/'/ ___   ____     ___   _ __    __
 `\ `\ /' / __`\/\_ ,`\  / __`\/\`'__\/'__`\
   `\ \ \/\ \L\ \/_/  /_/\ \L\ \ \ \//\ \L\.\_
     \ \_\ \____/ /\____\ \____/\ \_\\ \__/.\_\
      \/_/\/___/  \/____/\/___/  \/_/ \/__/\/_/
 ____    __              ___    ___
/\  _`\ /\ \            /\_ \  /\_ \
\ \,\L\_\ \ \___      __\//\ \ \//\ \
 \/_\__ \\ \  _ `\  /'__`\\ \ \  \ \ \
   /\ \L\ \ \ \ \ \/\  __/ \_\ \_ \_\ \_
   \ `\____\ \_\ \_\ \____\/\____\/\____\
    \/_____/\/_/\/_/\/____/\/____/\/____/
"#;

const SHELL_VERSION: &str = "yozora-shell v2.0 (user space)";
const INPUT_CAP: usize = 128;
const HISTORY_CAP: usize = 24;

// Scancodes for arrow keys
const SC_ENTER: u8 = 28;
const SC_BACKSPACE: u8 = 14;
const SC_UP: u8 = 72;
const SC_DOWN: u8 = 80;

struct YozoraShell {
    buf: Vec<char>,
    history: VecDeque<String>,
    hist_cursor: Option<usize>,
}

impl YozoraShell {
    fn init() -> Self {
        Self {
            buf: Vec::with_capacity(INPUT_CAP),
            history: VecDeque::with_capacity(HISTORY_CAP),
            hist_cursor: None,
        }
    }

    fn start(&mut self) {
        self.show_banner();
        self.draw_prompt();

        loop {
            let raw = usr_get_key();
            let scancode = ((raw >> 8) & 0xFF) as u8;
            let ascii = (raw & 0xFF) as u8;
            self.process_key(scancode, ascii);
        }
    }

    fn show_banner(&self) {
        usr_print(SHELL_BANNER);
        usr_set_color(WHITE);
        usr_print("\n");
        usr_print(SHELL_VERSION);
        usr_print("\nType `help` to see commands.\n\n");
    }

    fn draw_prompt(&self) {
        usr_set_color(HHU_BLUE);
        usr_print(PROMPT);
        usr_set_color(WHITE);
    }

    fn process_key(&mut self, scancode: u8, ascii: u8) {
        match scancode {
            SC_ENTER => {
                usr_print("\n");
                let input_line = self.collect_line();
                let trimmed = trim_owned(&input_line);
                if !trimmed.is_empty() {
                    self.push_history(&trimmed);
                    self.handle_line(&trimmed);
                }
                self.buf.clear();
                self.hist_cursor = None;
                self.draw_prompt();
            }
            SC_BACKSPACE => {
                if self.buf.pop().is_some() {
                    usr_print("\x08");
                }
            }
            SC_UP => {
                self.history_up();
            }
            SC_DOWN => {
                self.history_down();
            }
            _ => {
                if ascii != 0 && ascii < 0x80 {
                    let ch = ascii as char;
                    if (ch.is_ascii_graphic() || ch == ' ') && self.buf.len() < INPUT_CAP {
                        self.buf.push(ch);
                        let s = [ascii];
                        if let Ok(cs) = core::str::from_utf8(&s) {
                            usr_print(cs);
                        }
                    }
                }
            }
        }
    }

    fn collect_line(&self) -> String {
        self.buf.iter().cloned().collect()
    }

    fn push_history(&mut self, line: &str) {
        if self.history.back().map(|b| b.as_str()) != Some(line) {
            if self.history.len() == HISTORY_CAP {
                self.history.pop_front();
            }
            self.history.push_back(String::from(line));
        }
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if self.hist_cursor.is_none() {
            self.hist_cursor = Some(self.history.len() - 1);
        } else if let Some(idx) = self.hist_cursor {
            if idx > 0 {
                self.hist_cursor = Some(idx - 1);
            }
        }
        if let Some(idx) = self.hist_cursor {
            self.replace_line_with_history(idx);
        }
    }

    fn history_down(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if let Some(idx) = self.hist_cursor {
            if idx + 1 < self.history.len() {
                self.hist_cursor = Some(idx + 1);
                self.replace_line_with_history(idx + 1);
            } else {
                self.hist_cursor = None;
                self.clear_current_input_on_screen();
                self.buf.clear();
                self.draw_prompt();
            }
        }
    }

    fn replace_line_with_history(&mut self, idx: usize) {
        let entry = self.history.get(idx).cloned().unwrap_or_default();
        self.clear_current_input_on_screen();
        self.buf = entry.chars().collect();
        self.draw_prompt();
        let s: String = self.buf.iter().collect();
        usr_print(&s);
    }

    fn clear_current_input_on_screen(&self) {
        let cur_len = self.buf.len();
        for _ in 0..(PROMPT.len() + cur_len) {
            usr_print("\x08");
        }
    }

    fn handle_line(&self, line: &str) {
        let mut iter = line.split_whitespace();
        let cmd = match iter.next() {
            Some(c) => c,
            None => return,
        };
        let args: Vec<&str> = iter.collect();

        match cmd {
            "help" => self.cmd_help(),
            "clear" => self.cmd_clear(),
            "ver" | "version" => self.cmd_version(),
            "echo" => self.cmd_echo(&args),
            "tid" => self.cmd_tid(),
            "pid" => self.cmd_pid(),
            "uptime" => self.cmd_uptime(),
            "history" => self.cmd_history(),
            "vmas" => self.cmd_vmas(),
            "demo" => self.cmd_demo(&args),
            "exit" | "quit" => {
                usr_print("Goodbye.\n");
                usr_thread_exit();
            }
            name => self.cmd_run_external(name),
        }
    }

    fn cmd_help(&self) {
        usr_print("YozoraShell - User Space Shell\n");
        usr_print("Built-in commands:\n");
        usr_print("  help           Show this list\n");
        usr_print("  clear          Wipe the framebuffer\n");
        usr_print("  ver            Show shell version\n");
        usr_print("  echo <text>    Repeat text\n");
        usr_print("  tid            Current thread ID\n");
        usr_print("  pid            Current process ID\n");
        usr_print("  uptime         System uptime\n");
        usr_print("  history        List previous commands\n");
        usr_print("  vmas           Dump VMAs of this process\n");
        usr_print("  demo <name>    Run a demo (see list below)\n");
        usr_print("  exit|quit      Leave the shell\n");
        usr_print("\nDemo apps (via 'demo <name>'):\n");
        usr_print("  hello          Task 12/13 demo (heap, VMAs, stack)\n");
        usr_print("  uptime_app     System uptime (A9 syscalls)\n");
        usr_print("  vmatest        VMA & heap test (A13 features)\n");
        usr_print("\nType any app name directly to launch it.\n");
    }

    fn cmd_clear(&self) {
        usr_buff_clear();
    }

    fn cmd_version(&self) {
        usr_print(SHELL_VERSION);
        usr_print("\nBuilt for x86_64, written by Yozora.\n");
    }

    fn cmd_echo(&self, args: &[&str]) {
        if args.is_empty() {
            usr_print("\n");
        } else {
            for (i, a) in args.iter().enumerate() {
                if i > 0 { usr_print(" "); }
                usr_print(a);
            }
            usr_print("\n");
        }
    }

    fn cmd_tid(&self) {
        let tid = usr_thread_get_id();
        usr_print("Active TID: ");
        print_usize(tid);
        usr_print("\n");
    }

    fn cmd_pid(&self) {
        let pid = usr_get_process_id();
        usr_print("PID: ");
        print_usize(pid);
        usr_print("\n");
    }

    fn cmd_uptime(&self) {
        let ms = usr_get_system_time();
        let total_s = ms / 1000;
        let h = total_s / 3600;
        let m = (total_s % 3600) / 60;
        let s = total_s % 60;
        usr_print("Uptime: ");
        print_padded(h, 2); usr_print(":");
        print_padded(m, 2); usr_print(":");
        print_padded(s, 2);
        usr_print(" ("); print_usize(ms); usr_print(" ms)\n");
    }

    fn cmd_history(&self) {
        if self.history.is_empty() {
            usr_print("No entries in history.\n");
            return;
        }
        usr_print("History (most recent last):\n");
        for (i, e) in self.history.iter().enumerate() {
            usr_print("  ");
            print_usize(i + 1);
            usr_print(": ");
            usr_print(e);
            usr_print("\n");
        }
    }

    fn cmd_vmas(&self) {
        usr_dump_vmas();
    }

    fn cmd_demo(&self, args: &[&str]) {
        if args.is_empty() {
            usr_print("Demos: hello, uptime_app, vmatest\n");
            return;
        }
        self.cmd_run_external(args[0]);
    }

    fn cmd_run_external(&self, name: &str) {
        let pid = usr_spawn_process(name);
        if pid == 0 {
            usr_print("Unknown command: '");
            usr_print(name);
            usr_print("'\n");
            return;
        }
        usr_wait_pid(pid);
    }
}

fn print_usize(val: usize) {
    let mut buf = [0u8; 20];
    let len = write_decimal(val, &mut buf);
    if let Ok(s) = core::str::from_utf8(&buf[..len]) {
        usr_print(s);
    }
}

fn print_padded(val: usize, width: usize) {
    let mut buf = [0u8; 20];
    let len = write_decimal_padded(val, width, &mut buf);
    if let Ok(s) = core::str::from_utf8(&buf[..len]) {
        usr_print(s);
    }
}

fn write_decimal_padded(val: usize, width: usize, buf: &mut [u8]) -> usize {
    let mut digits = [0u8; 20];
    let mut n = val;
    let mut len = 0;
    if n == 0 {
        digits[0] = b'0';
        len = 1;
    } else {
        while n > 0 {
            digits[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
    }
    let pad = if width > len { width - len } else { 0 };
    for i in 0..pad { buf[i] = b'0'; }
    for i in 0..len { buf[pad + i] = digits[len - 1 - i]; }
    pad + len
}

fn write_decimal(val: usize, buf: &mut [u8]) -> usize {
    write_decimal_padded(val, 0, buf)
}

fn trim_owned(s: &str) -> String {
    String::from(s.trim())
}

#[unsafe(link_section = ".main")]
#[unsafe(no_mangle)]
fn main() {
    usr_map_heap(USER_HEAP_START, USER_HEAP_SIZE);
    unsafe {
        USER_ALLOCATOR.lock().init_at(USER_HEAP_START as usize, USER_HEAP_SIZE);
    }

    let mut shell = YozoraShell::init();
    shell.start();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    usr_print("Shell panic!\n");
    loop {}
}
