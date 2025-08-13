/// Native UTC DateTime implementation using chrono crate
#[derive(Clone, Debug)]
pub struct DateTime(chrono::DateTime<chrono::Utc>);

impl DateTime {
    #[cfg(not(target_arch = "wasm32"))]
    fn now() -> Self {
        Self(chrono::Utc::now())
    }

    #[cfg(target_arch = "wasm32")]
    fn now() -> Self {
        let now = js_sys::Date::new_0();
        Self(chrono::DateTime::from(now))
    }

    fn format(&self, format_str: &str) -> String {
        self.0.format(format_str).to_string()
    }
}

// Convenience functibon to get current timestamp formatted
pub fn format_utc_now(format_str: &str) -> String {
    DateTime::now().format(format_str)
}
