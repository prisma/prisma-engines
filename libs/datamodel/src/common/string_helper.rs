pub struct WritableString {
    inner: String,
}

impl WritableString {
    pub fn new() -> WritableString {
        WritableString { inner: "".to_string() }
    }

    pub fn into(self) -> String {
        self.inner
    }
}

impl std::io::Write for WritableString {
    fn write(&mut self, buf: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let as_string = String::from_utf8(buf.to_vec()).expect("ByteArray to String failed");
        self.inner.push_str(&as_string);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}
