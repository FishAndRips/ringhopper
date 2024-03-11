use std::fmt::Arguments;
use std::io::{BufWriter, Stdout, stdout, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use ringhopper::error::{Error, RinghopperResult};
use ringhopper::logger::Logger;

pub fn read_file<P: AsRef<Path>>(path: P) -> RinghopperResult<Vec<u8>> {
    let path = path.as_ref();
    std::fs::read(path).map_err(|e| Error::FailedToReadFile(path.to_owned(), e))
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
struct TTYMetadata {
    /// Number of columns.
    pub width: usize,

    /// Number of rows.
    pub height: usize,

    /// Color output is supported.
    pub color: bool
}

fn get_tty_metadata() -> Option<TTYMetadata> {
    if cfg!(target_os = "linux") {
        #[cfg(target_os = "linux")]
        {
            // Use the Linux API to get this
            let mut ws = libc::winsize { ws_col: 0, ws_row: 0, ws_xpixel: 0, ws_ypixel: 0 };
            let result = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws as *mut libc::winsize) };

            return if result == 0 {
                Some(TTYMetadata { width: ws.ws_col as usize, height: ws.ws_row as usize, color: true })
            }
            else {
                None
            }
        }
    }
    else if cfg!(target_os = "windows") {
        #[cfg(target_os = "windows")]
        {
            // Use the Windows API to get this
            use windows::Win32::System::Console;

            let mut w = Console::CONSOLE_SCREEN_BUFFER_INFO::default();
            let handle = unsafe { Console::GetStdHandle(Console::STD_OUTPUT_HANDLE).unwrap() };
            let is_terminal = unsafe { Console::GetConsoleScreenBufferInfo(handle, &mut w) }.is_ok();

            if !is_terminal {
                return None
            }

            // Now check if color is supported
            let mut console_mode = Console::CONSOLE_MODE::default();
            let color_support = if unsafe { Console::GetConsoleMode(handle, &mut console_mode).is_ok() } {
                if (console_mode & Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING).0 != 0 {
                    // We already have color support
                    true
                }
                else {
                    // Try enabling color support
                    unsafe { Console::SetConsoleMode(handle, console_mode | Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING) }.is_ok()
                }
            }
            else {
                false
            };

            return Some(TTYMetadata {
                width: w.srWindow.Right as usize - w.srWindow.Left as usize + 1,
                height: w.srWindow.Bottom as usize - w.srWindow.Top as usize + 1,
                color: color_support
            })
        }
    }
    else {
        return None
    }
    unreachable!();
}

pub fn make_stdout_logger() -> Arc<dyn Logger> {
    Arc::new(StdoutLogger {
        logger: Mutex::new(BufWriter::with_capacity(8 * 1024, stdout())),
        metadata: get_tty_metadata()
    })
}

struct StdoutLogger {
    logger: Mutex<BufWriter<Stdout>>,
    metadata: Option<TTYMetadata>
}

impl StdoutLogger {
    fn has_color(&self) -> bool {
        self.metadata.is_some_and(|m| m.color)
    }

    fn write_fmt(&self, fmt: Arguments, prefix: &str, postfix: &str) {
        let mut logger = self.logger.lock().unwrap();
        logger.write(prefix.as_bytes()).unwrap();
        logger.write_fmt(fmt).unwrap();
        logger.write(postfix.as_bytes()).unwrap();
    }
}

impl Logger for StdoutLogger {
    fn flush(&self) {
        self.logger.lock().unwrap().flush().unwrap()
    }

    fn neutral_fmt(&self, fmt: Arguments) {
        self.write_fmt(fmt, "", "");
    }

    fn success_fmt(&self, fmt: Arguments) {
        if !self.has_color() {
            return self.neutral_fmt_ln(fmt)
        }
        self.write_fmt(fmt, "\x1B[32m", "\x1B[m")
    }

    fn warning_fmt(&self, fmt: Arguments) {
        if !self.has_color() {
            return self.neutral_fmt_ln(fmt)
        }
        self.write_fmt(fmt, "\x1B[1;33m", "\x1B[m")
    }

    fn error_fmt(&self, fmt: Arguments) {
        if !self.has_color() {
            return self.neutral_fmt_ln(fmt)
        }
        self.write_fmt(fmt, "\x1B[1;31m", "\x1B[m")
    }
}
