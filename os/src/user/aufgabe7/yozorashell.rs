use crate::devices::{keyboard, lfb};
use crate::kernel::threads::scheduler::get_scheduler;
use crate::user::aufgabe2::sound_demo;
use crate::user::aufgabe7::graphic_demo;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::devices::lfb::{color, get_lfb, HHU_GREEN, HHU_BLUE};
use crate::kernel::threads::thread::Thread;
use crate::cga_print;
use alloc::collections::VecDeque;
use crate::kernel::threads::thread;
use crate::{buff_print, devices::buff_print::{buff_print, buff_clear, buff_set_color}};
use crate::devices::lfb::WHITE;


// Shell appearance / limits
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

const SHELL_VERSION: &str = "yozora-shell v0.1 (custom)";
const INPUT_CAP: usize = 128;
const HISTORY_CAP: usize = 24;

pub struct YozoraShell {
    // use Vec<char> for easy cursor/backspace manipulation
    buf: Vec<char>,
    // ring buffer style history: newest at back
    history: VecDeque<String>,
    // index for navigating history: None means "editing current buffer"
    hist_cursor: Option<usize>,
}


fn run_text_demo() {
    crate::user::aufgabe1::text_demo::run();
    buff_set_color(HHU_BLUE);
    buff_print!("yozora$ ");
    buff_set_color(WHITE);
}

fn run_keyboard_demo() {
    crate::user::aufgabe1::keyboard_demo::run();
    buff_set_color(HHU_BLUE);
    buff_print!("yozora$ ");
    buff_set_color(WHITE);
}

fn run_sound_demo() {
    crate::user::aufgabe2::sound_demo::run();
    buff_set_color(HHU_BLUE);
    buff_print!("yozora$ ");
    buff_set_color(WHITE);
}

fn run_heap_demo() {
    crate::user::aufgabe2::heap_demo::run();
    buff_set_color(HHU_BLUE);
    buff_print!("yozora$ ");
    buff_set_color(WHITE);
}

fn run_graphics_demo() {
    graphic_demo::run();
}

fn run_threads_demo() {
    crate::user::aufgabe4::thread_demo::run();
}

fn run_synchronize_demo() {
    crate::user::aufgabe7::synchronize_demo::run();
}

impl YozoraShell {
    pub fn init() -> Self {
        Self {
            buf: Vec::with_capacity(INPUT_CAP),
            history: VecDeque::with_capacity(HISTORY_CAP),
            hist_cursor: None,
        }
    }

    /// Start the shell main loop (blocks)
    /// now cooperative: yields to scheduler when idle and after processing a key
    pub fn start(&mut self) {
        self.show_banner();
        self.draw_prompt();

        loop {
            let key = keyboard::get_key_buffer().wait_for_key();
            if !key.valid() {
                // no key available: yield CPU so other threads (demos) run
                let _ = get_scheduler().yield_cpu();
                continue;
            }
            // process key
            self.process_key(key);

            // after handling key, give other threads a chance to run
            let _ = get_scheduler().yield_cpu();
        }
    }

    fn show_banner(&self) {
        buff_print!("\n{}\n", SHELL_BANNER);
        buff_print!("Type `help` to see commands.\n\n");
    }

    fn draw_prompt(&self) {
        buff_set_color(HHU_BLUE);
        buff_print!("{}", PROMPT);
        buff_set_color(lfb::WHITE);
    }

    fn process_key(&mut self, key: crate::devices::key::Key) {
        match key.get_scancode() {
            28 => { // Enter
                buff_print!("\n");
                let input_line = self.collect_line();
                self.handle_line(&input_line);
                self.buf.clear();
                self.hist_cursor = None;

                // only draw prompt if command is synchronous
                if !self.is_background_command(&input_line) {
                    self.draw_prompt();
                }
            }

            14 => { // Backspace
                if let Some(_) = self.buf.pop() {
                    // send backspace to LFB printing helper
                    buff_print!("\x08");
                }
            }
            72 => { // Up
                self.history_up();
            }
            80 => { // Down
                self.history_down();
            }
            _ => {
                let ascii = key.get_ascii();
                if ascii != 0 && ascii.is_ascii() {
                    let ch = ascii as u8 as char;
                    if (ch.is_ascii_graphic() || ch == ' ') && self.buf.len() < INPUT_CAP {
                        self.buf.push(ch);
                        buff_print!("{}", ch);
                    }
                }
            }
        }
    }
    fn is_background_command(&self, line: &str) -> bool {
        // split once to get command
        let cmd = line.split_whitespace().next().unwrap_or("");
        matches!(cmd, "demo" | "gfx" | "sound")
    }

    fn collect_line(&mut self) -> String {
        // trim leading/trailing whitespace
        let s: String = self.buf.iter().cloned().collect();
        s.trim().to_string()
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        // If not currently browsing history, start from the newest entry
        if self.hist_cursor.is_none() {
            self.hist_cursor = Some(self.history.len() - 1);
        } else {
            // move older if possible
            if let Some(idx) = self.hist_cursor {
                if idx > 0 {
                    self.hist_cursor = Some(idx - 1);
                }
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
                // past newest -> clear to empty editable buffer
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
        buff_print!("{}", self.buf.iter().collect::<String>());
    }

    fn clear_current_input_on_screen(&self) {
        // print backspaces for prompt + current input length
        let cur_len = self.buf.len();
        for _ in 0..(PROMPT.len() + cur_len) {
            buff_print!("\x08");
        }
    }

    fn handle_line(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }

        // push to history (avoid duplicate consecutive entries)
        if self.history.back().map(|b| b.as_str()) != Some(line) {
            if self.history.len() == HISTORY_CAP {
                self.history.pop_front();
            }
            self.history.push_back(line.to_string());
        }

        // split once on whitespace for command + rest
        let mut iter = line.split_whitespace();
        let cmd = iter.next().unwrap_or("");
        let args: Vec<&str> = iter.collect();

        match cmd {
            "help" => self.cmd_help(),
            "clear" => self.cmd_clear(),
            "ver" | "version" => self.cmd_version(),
            "echo" => self.cmd_echo(&args),
            "threads" => self.cmd_threads(),
            "uptime" => self.cmd_uptime(),
            "history" => self.cmd_history(),
            "graphics" => self.cmd_graphics(),
            "sound" => self.cmd_sound(&args),
            "demo" => self.cmd_demo(&args),
            "exit" | "quit" => {
                buff_print!("Goodbye.\n");

                loop {
                    // keep the shell alive; adjust as you prefer
                    thread::sleep_ms(1000);
                }
            }
            "" => {}
            other => {
                buff_print!("Unknown command: '{}'\n", other);
            }
        }
    }

    /* -------- command implementations (different text to your original) -------- */

    fn cmd_help(&self) {
        buff_print!("Commands (Yozora):\n");
        buff_print!("  help           show this list\n");
        buff_print!("  clear          wipe the framebuffer text\n");
        buff_print!("  version|ver    show shell & build info\n");
        buff_print!("  echo <text>    repeat text\n");
        buff_print!("  threads        quick thread info\n");
        buff_print!("  uptime         how long the system runs\n");
        buff_print!("  graphics       launch the graphical demo (if available)\n");
        buff_print!("  sound [name]   play built-in tunes\n");
        buff_print!("  demo <name>    run small demo programs\n");
        buff_print!("  history        list previous commands\n");
        buff_print!("  exit|quit      leave the shell\n");
    }

    fn cmd_clear(&self) {
        if lfb::is_lfb_initialized() {
            buff_clear();
        }
    }

    fn cmd_version(&self) {
        buff_print!("{}\n", SHELL_VERSION);
        buff_print!("Built for x86_64, written for Yozora.\n");
    }

    fn cmd_echo(&self, args: &[&str]) {
        if args.is_empty() {
            buff_print!("\n");
        } else {
            buff_print!("{}\n", args.join(" "));
        }
    }

    fn cmd_threads(&self) {
        let sched = get_scheduler();
        buff_print!("Active TID: {}\n", sched.get_active_tid());
    }

    fn cmd_uptime(&self) {
        use crate::devices::pit;
        let ms = pit::get_system_time();
        let s = ms / 1000;
        let h = s / 3600;
        let m = (s % 3600) / 60;
        let sec = s % 60;
        buff_print!("Uptime: {:02}:{:02}:{:02} ({} ms)\n", h, m, sec, ms);
    }

    fn cmd_history(&self) {
        if self.history.is_empty() {
            buff_print!("No entries in history.\n");
            return;
        }
        buff_print!("History (most recent last):\n");
        for (i, e) in self.history.iter().enumerate() {
            buff_print!("  {}: {}\n", i + 1, e);
        }
    }



    fn cmd_sound(&self, args: &[&str]) {
        // Run sound tunes in background threads so shell remains responsive
        if args.is_empty() {
            buff_print!("Sounds: tetris, aerodynamic\n");
            return;
        }
        match args[0] {
            "tetris" => {
                buff_print!("Queuing Tetris theme...\n");
                let t = thread::Thread::new(crate::devices::pcspk::tetris);
                get_scheduler().ready(t);
                self.draw_prompt();
            }
            "aerodynamic" => {
                buff_print!("Queuing Aerodynamic...\n");
                let t = thread::Thread::new(crate::devices::pcspk::aerodynamic);
                get_scheduler().ready(t);
                self.draw_prompt();
            }
            other => {
                buff_print!("Unknown tune: {}. Try without args to get list.\n", other);
            }
        }
    }

    fn cmd_graphics(&self) {
        if !lfb::is_lfb_initialized() {
            buff_print!("No framebuffer — graphics unavailable.\n");
            return;
        }
        let t = Thread::new(run_graphics_demo);
        get_scheduler().ready(t);
    }

    fn cmd_demo(&self, args: &[&str]) {
        if args.is_empty() {
            buff_print!("Demos: text, keyboard, heap, sound, graphics, threads, synchronize\n");
            self.draw_prompt();
            return;
        }

        match args[0] {
            "text" => {
                buff_print!("Launching text demo in background thread...\n");
                let t = crate::kernel::threads::thread::Thread::new(run_text_demo);
                get_scheduler().ready(t);
            }
            "keyboard" => {
                buff_print!("Launching keyboard demo synchronously (blocking)...\n");
                run_keyboard_demo();
            }
            "heap" => {
                buff_print!("Launching heap in background thread...\n");
                let t = crate::kernel::threads::thread::Thread::new(run_heap_demo);
                get_scheduler().ready(t);
            }
            "sound" => {
                buff_print!("Launching sound demo in background thread...\n");
                let t = crate::kernel::threads::thread::Thread::new(run_sound_demo);
                get_scheduler().ready(t);
            }
            "graphics" => {
                // This message is now correct. It calls cmd_graphics, which creates a thread.
                buff_print!("Launching graphics demo in background thread...\n");
                self.cmd_graphics();
            }
            "threads" => {
                buff_print!("Launching threads demo in background thread...\n");
                let t = crate::kernel::threads::thread::Thread::new(run_threads_demo);
                get_scheduler().ready(t);
            }
            "synchronize" =>{
                buff_print!("Launching mutex with queue demo in background thread...\n");
                let t = crate::kernel::threads::thread::Thread::new(run_synchronize_demo);
                get_scheduler().ready(t);
            }
            other => {
                buff_print!("Unknown demo: {}\n", other);
            }
        }
    }
}


/// convenience entrypoint used by your kernel
pub fn launch() {
    let mut s = YozoraShell::init();
    s.start();
}
