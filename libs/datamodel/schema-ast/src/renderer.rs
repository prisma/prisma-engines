mod table;

pub(crate) use table::TableFormat;

pub(crate) trait LineWriteable {
    fn write(&mut self, param: &str);
    fn end_line(&mut self);
}

pub(crate) struct Renderer {
    pub stream: String,
    indent: usize,
    indent_width: usize,
}

impl Renderer {
    pub(crate) fn new(indent_width: usize) -> Renderer {
        Renderer {
            stream: String::new(),
            indent: 0,
            indent_width,
        }
    }

    pub(crate) fn indent_up(&mut self) {
        self.indent += 1
    }

    pub(crate) fn indent_down(&mut self) {
        if self.indent == 0 {
            panic!("Indentation error.")
        }
        self.indent -= 1
    }
}

impl LineWriteable for Renderer {
    fn write(&mut self, param: &str) {
        if self.stream.is_empty() || self.stream.ends_with('\n') {
            for _ in 0..(self.indent * self.indent_width) {
                self.stream.push(' ');
            }
        }

        self.stream.push_str(param);
    }

    fn end_line(&mut self) {
        self.stream.push('\n');
    }
}

impl<'a> LineWriteable for &'a mut String {
    fn write(&mut self, param: &str) {
        self.push_str(param);
    }

    fn end_line(&mut self) {
        panic!("cannot end line in string builder");
    }
}
