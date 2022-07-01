use std::{fs, io, path::Path, sync::Arc};

#[derive(Debug, Clone)]
pub struct SourceFile {
    contents: Contents,
}

impl SourceFile {
    pub fn new_static(content: &'static str) -> Self {
        Self {
            contents: Contents::Static(content),
        }
    }

    pub fn new_allocated(s: Arc<Box<str>>) -> Self {
        Self {
            contents: Contents::Allocated(s),
        }
    }

    pub fn from_file(file: Box<Path>) -> io::Result<Self> {
        let data = fs::read_to_string(file.as_ref())?;

        let this = Self {
            contents: Contents::FromFile(Arc::new((file, data.into_boxed_str()))),
        };

        Ok(this)
    }

    pub fn as_str(&self) -> &str {
        match self.contents {
            Contents::Static(s) => s,
            Contents::FromFile(ref from_file) => from_file.1.as_ref(),
            Contents::Allocated(ref s) => &*s,
        }
    }
}

#[derive(Debug, Clone)]
enum Contents {
    Static(&'static str),
    Allocated(Arc<Box<str>>),
    FromFile(Arc<(Box<Path>, Box<str>)>),
}
