use crate::SelectionResult;

use super::Filter;

/// A wrapper struct allowing to either filter for records or for the core to
/// communicate already known record selectors to connectors.
///
/// Connector implementations should use known selectors to skip unnecessary fetch operations
/// if the query core already determined the selectors in a previous step. Simply put,
/// `selectors` should always have precendence over `filter`.
#[derive(Debug, Clone)]
pub struct RecordFilter {
    pub filter: Filter,
    pub selectors: Option<Vec<SelectionResult>>,
}

impl RecordFilter {
    pub fn empty() -> Self {
        Self {
            filter: Filter::empty(),
            selectors: None,
        }
    }

    pub fn has_selectors(&self) -> bool {
        self.selectors.is_some()
    }
}

impl From<Filter> for RecordFilter {
    fn from(filter: Filter) -> Self {
        Self {
            filter,
            selectors: None,
        }
    }
}

impl From<Vec<SelectionResult>> for RecordFilter {
    fn from(selectors: Vec<SelectionResult>) -> Self {
        Self {
            filter: Filter::empty(),
            selectors: Some(selectors),
        }
    }
}

impl From<SelectionResult> for RecordFilter {
    fn from(selector: SelectionResult) -> Self {
        Self {
            filter: Filter::empty(),
            selectors: Some(vec![selector]),
        }
    }
}
