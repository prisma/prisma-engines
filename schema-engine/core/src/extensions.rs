use bon::bon;
use hashbrown::{Equivalent, HashMap};
use psl::parser_database::{ExtensionTypeEntry, ExtensionTypeId, ExtensionTypes};
use serde::Deserialize;

/// Configuration for extension types.
#[derive(Debug, Default, Deserialize)]
#[serde(from = "ExtensionTypeConfigJson")]
pub struct ExtensionTypeConfig {
    types: Vec<ExtensionType>,
    by_prisma_name: HashMap<String, usize>,
    by_db_name_and_modifiers: HashMap<ExtensionTypeDbKey, usize>,
}

impl ExtensionTypeConfig {
    /// Create a new `ExtensionTypeConfig` from a list of `ExtensionType`.
    pub fn new(types: Vec<ExtensionType>) -> Self {
        let mut by_prisma_name = HashMap::new();
        let mut by_db_name_and_modifiers = HashMap::new();

        for (i, ext) in types.iter().enumerate() {
            by_prisma_name.insert(ext.prisma_name.clone(), i);
            let db_key = ExtensionTypeDbKey(ext.db_name.clone(), ext.db_type_modifiers.clone());
            by_db_name_and_modifiers.insert(db_key, i);
        }

        Self {
            types,
            by_prisma_name,
            by_db_name_and_modifiers,
        }
    }
}

impl ExtensionTypes for ExtensionTypeConfig {
    fn get_by_prisma_name(&self, name: &str) -> Option<ExtensionTypeId> {
        self.by_prisma_name.get(name).map(|&i| ExtensionTypeId::from(i))
    }

    fn get_by_db_name_and_modifiers(&self, name: &str, modifiers: Option<&[String]>) -> Option<ExtensionTypeEntry<'_>> {
        self.by_db_name_and_modifiers
            .get(&(name, modifiers))
            .or_else(|| self.by_db_name_and_modifiers.get(&(name, None)))
            .map(|&i| self.types[i].entry(i))
    }

    fn enumerate(&self) -> Box<dyn Iterator<Item = psl::parser_database::ExtensionTypeEntry<'_>> + '_> {
        Box::new(self.types.iter().enumerate().map(|(i, ext)| ext.entry(i)))
    }
}

/// Represents a single extension type.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionType {
    prisma_name: String,
    db_name: String,
    db_namespace: Option<String>,
    db_type_modifiers: Option<Vec<String>>,
    number_of_db_type_modifiers: usize,
}

#[bon]
impl ExtensionType {
    /// Create a new `ExtensionType`.
    #[builder]
    pub fn new(
        #[builder(into)] prisma_name: String,
        #[builder(into)] db_name: String,
        #[builder(into)] db_namespace: Option<String>,
        db_type_modifiers: Option<Vec<String>>,
        number_of_db_type_modifiers: usize,
    ) -> Self {
        Self {
            prisma_name,
            db_name,
            db_namespace,
            db_type_modifiers,
            number_of_db_type_modifiers,
        }
    }

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
        ExtensionTypeConfig::new(json.types)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct ExtensionTypeDbKey(String, Option<Vec<String>>);

impl Equivalent<ExtensionTypeDbKey> for (&str, Option<&[String]>) {
    fn equivalent(&self, key: &ExtensionTypeDbKey) -> bool {
        self.0 == key.0 && self.1 == key.1.as_deref()
    }
}
