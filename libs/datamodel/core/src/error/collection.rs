use super::DatamodelError;

/// Represents a list of validation or parser errors.
///
/// This is used to accumulate multiple errors during validation.
/// It is used to not error out early and instead show multiple errors at once.
#[derive(Debug, Clone)]
pub struct ErrorCollection {
    pub errors: Vec<DatamodelError>,
}

impl ErrorCollection {
    pub fn new() -> ErrorCollection {
        ErrorCollection { errors: Vec::new() }
    }

    pub fn push(&mut self, err: DatamodelError) {
        self.errors.push(err)
    }

    pub fn push_opt(&mut self, err: Option<DatamodelError>) {
        if let Some(err) = err {
            self.push(err);
        }
    }

    /// Returns true, if there is at least one error
    /// in this collection.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Creates an iterator over all errors in this collection.
    pub fn to_iter(&self) -> std::slice::Iter<DatamodelError> {
        self.errors.iter()
    }

    /// Appends all errors from another collection to this collection.
    pub fn append(&mut self, errs: &mut ErrorCollection) {
        self.errors.append(&mut errs.errors)
    }

    pub fn append_vec(&mut self, errors: Vec<DatamodelError>) {
        let mut errors = errors;
        self.errors.append(&mut errors);
    }

    pub fn ok(&self) -> Result<(), ErrorCollection> {
        if self.has_errors() {
            Err(self.clone())
        } else {
            Ok(())
        }
    }

    pub fn to_pretty_string(&self, file_name: &str, datamodel_string: &str) -> String {
        let mut message: Vec<u8> = Vec::new();

        for err in self.to_iter() {
            err.pretty_print(&mut message, file_name, datamodel_string)
                .expect("printing datamodel error");
        }

        String::from_utf8_lossy(&message).into_owned()
    }
}

impl std::fmt::Display for ErrorCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let msg: Vec<String> = self.errors.iter().map(|e| e.to_string()).collect();
        f.write_str(&msg.join("\n"))
    }
}

impl From<DatamodelError> for ErrorCollection {
    fn from(error: DatamodelError) -> Self {
        let mut col = ErrorCollection::new();
        col.push(error);
        col
    }
}
