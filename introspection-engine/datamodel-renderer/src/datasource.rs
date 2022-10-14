use core::fmt;
use std::default::Default;

use psl::datamodel_connector::RelationMode;

use crate::{Array, Commented, Env, Text, Value};

/// The datasource block in a PSL file.
#[derive(Debug)]
pub struct Datasource<'a> {
    name: &'a str,
    provider: Text<'a>,
    url: Env<'a>,
    shadow_database_url: Option<Env<'a>>,
    relation_mode: Option<RelationMode>,
    custom_properties: Vec<(&'a str, Value<'a>)>,
    documentation: Option<Commented<'a>>,
    namespaces: Array<'a>,
}

impl<'a> Datasource<'a> {
    /// Create a new datasource with all required properties.
    pub fn new(name: &'a str, provider: &'a str, url: impl Into<Env<'a>>) -> Self {
        Self {
            name,
            provider: Text(provider),
            url: url.into(),
            shadow_database_url: None,
            relation_mode: None,
            custom_properties: Default::default(),
            documentation: None,
            namespaces: Array::default(),
        }
    }

    /// Used for migration engine to reflect the contents of
    /// migrations directory to diff against the actual database.
    pub fn shadow_database_url(&mut self, url: impl Into<Env<'a>>) {
        self.shadow_database_url = Some(url.into());
    }

    /// Who handles referential integrity.
    pub fn relation_mode(&mut self, relation_mode: RelationMode) {
        self.relation_mode = Some(relation_mode);
    }

    /// Add a custom connector-specific property to the datasource.
    pub fn push_custom_property(&mut self, key: &'a str, value: impl Into<Value<'a>>) {
        self.custom_properties.push((key, value.into()));
    }

    /// The documentation on top of the datasource.
    pub fn documentation(&mut self, documentation: &'a str) {
        self.documentation = Some(Commented::Documentation(documentation));
    }

    /// Create a rendering from a PSL datasource.
    pub fn from_psl(psl_ds: &'a psl::Datasource) -> Self {
        let shadow_database_url = psl_ds.shadow_database_url.as_ref().map(|(url, _)| Env::from(url));

        Self {
            name: &psl_ds.name,
            provider: Text(&psl_ds.provider),
            url: Env::from(&psl_ds.url),
            shadow_database_url,
            relation_mode: psl_ds.relation_mode,
            documentation: psl_ds.documentation.as_deref().map(Commented::Documentation),
            custom_properties: Default::default(),
            namespaces: Array(psl_ds.namespaces.iter().map(|(ns, _)| Text(ns).into()).collect()),
        }
    }
}

impl<'a> fmt::Display for Datasource<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref doc) = self.documentation {
            doc.fmt(f)?;
        }

        writeln!(f, "datasource {} {{", self.name)?;
        writeln!(f, "provider = {}", self.provider)?;
        writeln!(f, "url = {}", self.url)?;

        if let Some(url) = self.shadow_database_url {
            writeln!(f, "shadowDatabaseUrl = {}", url)?;
        }

        if let Some(relation_mode) = self.relation_mode {
            writeln!(f, "relationMode = \"{}\"", relation_mode)?;
        }

        for (key, value) in self.custom_properties.iter() {
            writeln!(f, "{key} = {value}")?;
        }

        if !self.namespaces.is_empty() {
            writeln!(f, "schemas = {}", self.namespaces)?;
        }

        f.write_str("}\n")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use expect_test::expect;
    use psl::datamodel_connector::RelationMode;

    #[test]
    fn kitchen_sink() {
        let mut datasource = Datasource::new("db", "postgres", Env::variable("DATABASE_URL"));

        datasource.documentation("Here comes the sun king...\n\nEverybody's laughing,\nEverybody's happy!");
        datasource.shadow_database_url(Env::variable("SHADOW_DATABASE_URL"));
        datasource.relation_mode(RelationMode::ForeignKeys);

        let mut fun = Function::new("uuid_ossp");
        fun.push_param(("map", Text("uuid-ossp")));

        let mut extensions = Array::default();
        extensions.push("postgis");
        extensions.push(fun);

        datasource.push_custom_property("extensions", extensions);

        let expected = expect![[r#"
            /// Here comes the sun king...
            ///
            /// Everybody's laughing,
            /// Everybody's happy!
            datasource db {
              provider          = "postgres"
              url               = env("DATABASE_URL")
              shadowDatabaseUrl = env("SHADOW_DATABASE_URL")
              relationMode      = "foreignKeys"
              extensions        = [postgis, uuid_ossp(map: "uuid-ossp")]
            }
        "#]];

        let rendered = psl::reformat(&format!("{datasource}"), 2).unwrap();
        expected.assert_eq(&rendered)
    }
}
