use crate::{configuration, dml};
use std::collections::HashMap;

/// A validator to see if the given constraint name is used anywhere else in the
/// data model.
#[derive(Debug)]
pub(crate) struct NamesValidator<'dml> {
    seen: HashMap<&'dml str, usize>,
}

impl<'dml> NamesValidator<'dml> {
    pub(crate) fn new(schema: &'dml dml::Datamodel, source: Option<&configuration::Datasource>) -> Self {
        let mut seen: HashMap<&'dml str, usize> = HashMap::new();

        let enabled = source
            .map(|source| source.active_connector.supports_named_default_values())
            .unwrap_or(false);

        if enabled {
            for model in schema.models() {
                if let Some(name) = model.primary_key.as_ref().and_then(|pk| pk.db_name.as_ref()) {
                    let counter = seen.entry(name).or_insert(0);
                    *counter += 1;
                }

                for name in model
                    .relation_fields()
                    .filter_map(|rf| rf.relation_info.fk_name.as_ref())
                {
                    let counter = seen.entry(name).or_insert(0);
                    *counter += 1;
                }

                for name in model
                    .scalar_fields()
                    .filter_map(|sf| sf.default_value().and_then(|d| d.db_name()))
                {
                    let counter = seen.entry(name).or_insert(0);
                    *counter += 1;
                }
            }
        }

        Self { seen }
    }

    /// True, if more than one constrains has the given name.
    pub(crate) fn is_duplicate(&self, name: &str) -> bool {
        self.seen.get(name).map(|counter| *counter > 1).unwrap_or(false)
    }
}
