pub struct ConversionFailure {
    pub from: &'static str,
    pub to: &'static str,
}

impl ConversionFailure {
    pub fn new(from: &'static str, to: &'static str) -> ConversionFailure {
        ConversionFailure { from, to }
    }
}
