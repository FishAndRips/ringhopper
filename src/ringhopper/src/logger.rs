/// General logging trait for diagnostic output.
pub trait Logger: Send + Sync {
    /// Flush any buffered output now.
    fn flush(&self);

    /// Print formatted neutral text.
    ///
    /// This is typically displayed as the terminal's default colors (such as white or black).
    fn neutral_fmt(&self, fmt: std::fmt::Arguments);

    /// Print formatted success text.
    ///
    /// This is typically displayed as green.
    fn success_fmt(&self, fmt: std::fmt::Arguments);

    /// Print formatted warning text.
    ///
    /// This is typically displayed as yellow or orange.
    fn warning_fmt(&self, fmt: std::fmt::Arguments);

    /// Print formatted error text.
    ///
    /// This is typically displayed as red.
    fn error_fmt(&self, fmt: std::fmt::Arguments);

    /// Print formatted neutral text on a line.
    ///
    /// This is typically displayed as the terminal's default colors (such as white or black).
    fn neutral_fmt_ln(&self, fmt: std::fmt::Arguments) {
        self.neutral_fmt(format_args!("{fmt}\n"));
    }

    /// Print formatted success text on a line.
    ///
    /// This is typically displayed as green.
    fn success_fmt_ln(&self, fmt: std::fmt::Arguments) {
        self.success_fmt(format_args!("{fmt}\n"));
    }

    /// Print formatted warning text on a line.
    ///
    /// This is typically displayed as yellow or orange.
    fn warning_fmt_ln(&self, fmt: std::fmt::Arguments) {
        self.warning_fmt(format_args!("{fmt}\n"));
    }

    /// Print formatted error text on a line.
    ///
    /// This is typically displayed as red.
    fn error_fmt_ln(&self, fmt: std::fmt::Arguments) {
        self.error_fmt(format_args!("{fmt}\n"));
    }

    /// Print unformatted neutral text.
    ///
    /// This is typically displayed as the terminal's default colors (such as white or black).
    fn neutral(&self, str: &str) {
        self.neutral_fmt(format_args!("{str}"))
    }

    /// Print unformatted success text.
    ///
    /// This is typically displayed as green.
    fn success(&self, str: &str) {
        self.success_fmt(format_args!("{str}"))
    }

    /// Print unformatted warning text.
    ///
    /// This is typically displayed as yellow or orange.
    fn warning(&self, str: &str) {
        self.warning_fmt(format_args!("{str}"))
    }

    /// Print unformatted error text.
    ///
    /// This is typically displayed as red.
    fn error(&self, str: &str) {
        self.error_fmt(format_args!("{str}"))
    }

    /// Print unformatted neutral text on a line.
    ///
    /// This is typically displayed as the terminal's default colors (such as white or black).
    fn neutral_ln(&self, str: &str) {
        self.neutral_fmt_ln(format_args!("{str}"))
    }

    /// Print unformatted success text on a line.
    ///
    /// This is typically displayed as green.
    fn success_ln(&self, str: &str) {
        self.success_fmt_ln(format_args!("{str}"))
    }

    /// Print unformatted warning text on a line.
    ///
    /// This is typically displayed as yellow or orange.
    fn warning_ln(&self, str: &str) {
        self.warning_fmt_ln(format_args!("{str}"))
    }

    /// Print unformatted error text on a line.
    ///
    /// This is typically displayed as red.
    fn error_ln(&self, str: &str) {
        self.error_fmt_ln(format_args!("{str}"))
    }
}
