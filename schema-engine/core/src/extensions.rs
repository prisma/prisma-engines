use std::collections::HashMap;

use psl::parser_database::{ExtensionTypeEntry, ExtensionTypeId, ExtensionTypes};
use serde::Deserialize;

/// Configuration for extension types.
#[derive(Debug, Default, Deserialize)]
#[serde(from = "ExtensionTypeConfigJson")]
pub struct ExtensionTypeConfig {
    types: Vec<ExtensionType>,
    by_prisma_name: HashMap<String, usize>,
    by_db_name_and_modifiers: HashMap<(String, Option<Vec<String>>), usize>,
}

impl ExtensionTypes for ExtensionTypeConfig {
    fn get_by_prisma_name(&self, name: &str) -> Option<ExtensionTypeId> {
        self.by_prisma_name.get(name).map(|&i| ExtensionTypeId::from(i))
    }

    fn get_by_db_name_and_modifiers(&self, name: &str, modifiers: Option<&[String]>) -> Option<ExtensionTypeEntry<'_>> {
        self.by_db_name_and_modifiers
            .get(&(name.to_string(), modifiers.map(|m| m.to_vec())))
            .or_else(|| self.by_db_name_and_modifiers.get(&(name.to_string(), None)))
            .map(|&i| self.types[i].entry(i))
    }

    fn enumerate(&self) -> Box<dyn Iterator<Item = psl::parser_database::ExtensionTypeEntry<'_>> + '_> {
        Box::new(self.types.iter().enumerate().map(|(i, ext)| ext.entry(i)))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtensionType {
    prisma_name: String,
    db_name: String,
    db_namespace: Option<String>,
    db_type_modifiers: Option<Vec<String>>,
    number_of_db_type_modifiers: usize,
}

impl ExtensionType {
    fn entry(&self, id: usize) -> ExtensionTypeEntry<'_> {
        ExtensionTypeEntry {
            id: ExtensionTypeId::from(id),
            prisma_name: self.prisma_name.as_str(),
            db_name: self.db_name.as_str(),
            db_namespace: self.db_namespace.as_deref(),
            number_of_db_type_modifiers: self.number_of_db_type_modifiers,
            db_type_modifiers: self.db_type_modifiers.as_deref(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ExtensionTypeConfigJson {
    types: Vec<ExtensionType>,
}

impl From<ExtensionTypeConfigJson> for ExtensionTypeConfig {
    fn from(json: ExtensionTypeConfigJson) -> Self {
        let mut by_prisma_name = HashMap::new();
        let mut by_db_name_and_modifiers = HashMap::new();

        for (i, ext) in json.types.iter().enumerate() {
            by_prisma_name.insert(ext.prisma_name.clone(), i);
            by_db_name_and_modifiers.insert((ext.db_name.clone(), ext.db_type_modifiers.clone()), i);
        }

        Self {
            types: json.types,
            by_prisma_name,
            by_db_name_and_modifiers,
        }
    }
}
