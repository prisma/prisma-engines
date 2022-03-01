use super::LineWriteable;

pub struct StringBuilder {
    buffer: Vec<String>,
}

impl StringBuilder {
    pub fn new() -> StringBuilder {
        Self { buffer: Vec::new() }
    }
}

impl ToString for StringBuilder {
    fn to_string(&self) -> String {
        self.buffer.join("")
    }
}

impl LineWriteable for StringBuilder {
    fn write(&mut self, text: &str) {
        self.buffer.push(String::from(text));
    }

    fn end_line(&mut self) {
        unimplemented!("Cannot render new line in string builder.")
    }

    fn maybe_end_line(&mut self) {
        unimplemented!("Cannot render new line in string builder.")
    }

    fn line_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl Default for StringBuilder {
    fn default() -> Self {
        Self::new()
    }
}
