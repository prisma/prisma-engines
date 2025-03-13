/// A DateTime representation in UTC timezone.
pub trait UtcDateTime {
    /// Get current UTC time.
    fn now() -> Self
    where
        Self: Sized;

    /// Format datetime using a format string following strftime patterns.
    ///
    /// Common patterns:
    /// - %Y: Year with century (e.g., 2023)
    /// - %m: Month (01-12)
    /// - %d: Day of month (01-31)
    /// - %H: Hour (00-23)
    /// - %M: Minute (00-59)
    /// - %S: Second (00-59)
    fn format(&self, format_str: &str) -> String;
}
