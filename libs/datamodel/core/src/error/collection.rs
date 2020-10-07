use super::DatamodelError;
use crate::error::error::DatamodelError;
use crate::error::warning::DatamodelWarning;

/// Represents a list of validation or parser errors and warnings.
///
/// This is used to accumulate multiple errors and warnings during validation.
/// It is used to not error out early and instead show multiple errors at once.
#[derive(Debug, Clone)]
pub struct MessageCollection {
    pub errors: Vec<DatamodelError>,
    pub warnings: Vec<DatamodelWarning>
}

impl MessageCollection {
    pub fn new() -> MessageCollection {
        MessageCollection { errors: Vec::new(), warnings: Vec::new() }
    }

    pub fn push_error(&mut self, err: DatamodelError) {
        self.errors.push(err)
    }

    pub fn push_warning(&mut self, warning: DatamodelWarning) { self.warnings.push(warning) }

    pub fn push_error_opt(&mut self, err: Option<DatamodelError>) {
        match err {
            Some(err) => self.push_error(err),
            None => {}
        }
    }

    /// Returns true, if there is at least one error
    /// in this collection.
    pub fn has_errors(&self) -> bool {
        self.errors.len() > 0
    }

    pub fn has_warnings(&self) -> bool {self.warnings.len() > 0}

    /// Creates an iterator over all errors in this collection.
    pub fn to_error_iter(&self) -> std::slice::Iter<DatamodelError> {
        self.errors.iter()
    }

    /// Appends all errors and warnings from another collection to this collection.
    pub fn append(&mut self, messages: &mut MessageCollection) {
        self.errors.append(&mut messages.errors);
        self.warnings.append(&mut messages.warnings)
    }

    pub fn append_vec(&mut self, errors: Vec<DatamodelError>) {
        let mut errors = errors;
        self.errors.append(&mut errors);
    }

    pub fn ok(&self) -> Result<(), MessageCollection> {
        if self.has_errors() {
            Err(self.clone())
        } else {
            Ok(())
        }
    }

    pub fn to_pretty_string(&self, file_name: &str, datamodel_string: &str) -> String {
        let mut message: Vec<u8> = Vec::new();

        for err in self.to_error_iter() {
            err.pretty_print(&mut message, file_name, datamodel_string)
                .expect("printing datamodel error");
        }

        String::from_utf8_lossy(&message).into_owned()
    }
}

impl std::fmt::Display for MessageCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg: Vec<String> = self.errors.iter().map(|e| e.to_string()).collect();
        f.write_str(&msg.join("\n"))
    }
}

impl From<DatamodelError> for MessageCollection {
    fn from(error: DatamodelError) -> Self {
        let mut col = MessageCollection::new();
        col.push_error(error);
        col
    }
}

impl From<DatamodelWarning> for MessageCollection {
    fn from(warning: DatamodelWarning) -> Self {
        let mut col = MessageCollection::new();
        col.push_warning(warning);
        col
    }
}
