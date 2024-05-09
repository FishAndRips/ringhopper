#![allow(dead_code)]

use std::fmt::Arguments;
use std::io::{BufWriter, Stdout, stdout, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use ringhopper::error::{Error, RinghopperResult};

pub fn read_file<P: AsRef<Path>>(path: P) -> RinghopperResult<Vec<u8>> {
    let path = path.as_ref();
    std::fs::read(path).map_err(|e| Error::FailedToReadFile(path.to_owned(), e))
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct TTYMetadata {
    /// Number of columns.
    pub width: usize,

    /// Number of rows.
    pub height: usize,

    /// Color output is supported.
    pub color: bool
}

impl Default for TTYMetadata {
    fn default() -> Self {
        Self {
            width: 80,
            height: 24,
            color: false
        }
    }
}

/// Get the metadata for the current shell.
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub fn get_tty_metadata() -> Option<TTYMetadata> {
    // Fallback
    None
}

/// Get the metadata for the current shell.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn get_tty_metadata() -> Option<TTYMetadata> {
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

/// Get the metadata for the current shell.
#[cfg(target_os = "windows")]
pub fn get_tty_metadata() -> Option<TTYMetadata> {
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

pub fn make_stdout_logger() -> Arc<StdoutLogger> {
    Arc::new(StdoutLogger {
        logger: Mutex::new(BufWriter::with_capacity(8 * 1024, stdout())),
        metadata: get_tty_metadata()
    })
}

macro_rules! make_logger_impl {
    () => {
        /// Flush any buffered output now.
        pub fn flush(&self) {
            self.logger.lock().unwrap().flush().unwrap()
        }

        /// Print formatted neutral text.
        ///
        /// This is typically displayed as the terminal's default colors (such as white or black).
        pub fn neutral_fmt(&self, fmt: Arguments) {
            self.write_fmt(fmt, "", "");
        }

        /// Print formatted success text.
        ///
        /// This is typically displayed as green.
        pub fn success_fmt(&self, fmt: Arguments) {
            if !self.has_color() {
                return self.neutral_fmt(fmt)
            }
            self.write_fmt(fmt, "\x1B[32m", "\x1B[m")
        }

        /// Print formatted minor/pedantic warning text.
        ///
        /// This is typically displayed as purple.
        pub fn minor_warning_fmt(&self, fmt: Arguments) {
            if !self.has_color() {
                return self.neutral_fmt(fmt)
            }
            self.write_fmt(fmt, "\x1B[1;35m", "\x1B[m")
        }

        /// Print formatted warning text.
        ///
        /// This is typically displayed as yellow or orange.
        pub fn warning_fmt(&self, fmt: Arguments) {
            if !self.has_color() {
                return self.neutral_fmt(fmt)
            }
            self.write_fmt(fmt, "\x1B[1;33m", "\x1B[m")
        }

        /// Print formatted error text.
        ///
        /// This is typically displayed as red.
        pub fn error_fmt(&self, fmt: Arguments) {
            if !self.has_color() {
                return self.neutral_fmt(fmt)
            }
            self.write_fmt(fmt, "\x1B[1;31m", "\x1B[m")
        }

        /// Print formatted neutral text on a line.
        ///
        /// This is typically displayed as the terminal's default colors (such as white or black).
        pub fn neutral_fmt_ln(&self, fmt: std::fmt::Arguments) {
            self.neutral_fmt(format_args!("{fmt}\n"));
        }

        /// Print formatted success text on a line.
        ///
        /// This is typically displayed as green.
        pub fn success_fmt_ln(&self, fmt: std::fmt::Arguments) {
            self.success_fmt(format_args!("{fmt}\n"));
        }

        /// Print formatted minor/pedantic warning text on a line.
        ///
        /// This is typically displayed as purple.
        pub fn minor_warning_fmt_ln(&self, fmt: std::fmt::Arguments) {
            self.minor_warning_fmt(format_args!("{fmt}\n"));
        }

        /// Print formatted warning text on a line.
        ///
        /// This is typically displayed as yellow or orange.
        pub fn warning_fmt_ln(&self, fmt: std::fmt::Arguments) {
            self.warning_fmt(format_args!("{fmt}\n"));
        }

        /// Print formatted error text on a line.
        ///
        /// This is typically displayed as red.
        pub fn error_fmt_ln(&self, fmt: std::fmt::Arguments) {
            self.error_fmt(format_args!("{fmt}\n"));
        }

        /// Print unformatted neutral text.
        ///
        /// This is typically displayed as the terminal's default colors (such as white or black).
        pub fn neutral(&self, str: &str) {
            self.neutral_fmt(format_args!("{str}"))
        }

        /// Print unformatted success text.
        ///
        /// This is typically displayed as green.
        pub fn success(&self, str: &str) {
            self.success_fmt(format_args!("{str}"))
        }

        /// Print unformatted minor/pedantic warning text.
        ///
        /// This is typically displayed as purple.
        pub fn minor_warning(&self, str: &str) {
            self.minor_warning_fmt(format_args!("{str}"))
        }

        /// Print unformatted warning text.
        ///
        /// This is typically displayed as yellow or orange.
        pub fn warning(&self, str: &str) {
            self.warning_fmt(format_args!("{str}"))
        }

        /// Print unformatted error text.
        ///
        /// This is typically displayed as red.
        pub fn error(&self, str: &str) {
            self.error_fmt(format_args!("{str}"))
        }

        /// Print unformatted neutral text on a line.
        ///
        /// This is typically displayed as the terminal's default colors (such as white or black).
        pub fn neutral_ln(&self, str: &str) {
            self.neutral_fmt_ln(format_args!("{str}"))
        }

        /// Print unformatted success text on a line.
        ///
        /// This is typically displayed as green.
        pub fn success_ln(&self, str: &str) {
            self.success_fmt_ln(format_args!("{str}"))
        }

        /// Print unformatted warning text on a line.
        ///
        /// This is typically displayed as yellow or orange.
        pub fn warning_ln(&self, str: &str) {
            self.warning_fmt_ln(format_args!("{str}"))
        }

        /// Print unformatted error text on a line.
        ///
        /// This is typically displayed as red.
        pub fn error_ln(&self, str: &str) {
            self.error_fmt_ln(format_args!("{str}"))
        }
    };
}

pub struct StdoutLogger {
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

    /// Lock the logger.
    ///
    /// Use this to guarantee multiple logs are contained together in the console.
    pub fn lock(&self) -> LockedStdoutLogger {
        LockedStdoutLogger {
            logger: Mutex::new(self.logger.lock().unwrap()),
            metadata: self.metadata.clone()
        }
    }
}

pub struct LockedStdoutLogger<'a> {
    logger: Mutex<MutexGuard<'a, BufWriter<Stdout>>>,
    metadata: Option<TTYMetadata>
}

impl<'a> LockedStdoutLogger<'a> {
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

impl StdoutLogger {
    make_logger_impl!();
}

impl<'a> LockedStdoutLogger<'a> {
    make_logger_impl!();
}
