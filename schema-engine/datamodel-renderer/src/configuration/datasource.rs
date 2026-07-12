use crate::value::{Array, Documentation, Text, Value};
use core::fmt;
use psl::datamodel_connector::RelationMode;
use std::{borrow::Cow, default::Default};

/// The datasource block in a PSL file.
#[derive(Debug)]
pub struct Datasource<'a> {
    name: &'a str,
    provider: Text<&'a str>,
    relation_mode: Option<RelationMode>,
    custom_properties: Vec<(&'a str, Value<'a>)>,
    documentation: Option<Documentation<'a>>,
    namespaces: Array<Text<Cow<'a, str>>>,
}

impl<'a> Datasource<'a> {
    /// Create a new datasource with all required properties.
    ///
    /// ```ignore
    /// datasource db {
    /// //         ^^ name
    ///   provider = "postgresql"
    /// //            ^^^^^^^^^^ provider
    /// }
    /// ```
    pub fn new(name: &'a str, provider: &'a str) -> Self {
        Self {
            name,
            provider: Text(provider),
            relation_mode: None,
            custom_properties: Default::default(),
            documentation: None,
            namespaces: Array::new(),
        }
    }

    /// Who handles referential integrity.
    ///
    /// ```ignore
    /// datasource db {
    ///   provider     = "postgresql"
    ///   relationMode = "foreignKeys"
    /// //                ^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn relation_mode(&mut self, relation_mode: RelationMode) {
        self.relation_mode = Some(relation_mode);
    }

    /// Add a custom connector-specific property to the datasource.
    /// Use this for settings that are only for a single database. For
    /// shared things, add an explicit property to the `Datasource`
    /// struct.
    ///
    /// An example for PostgreSQL extensions, using an array of functions:
    ///
    /// ```ignore
    /// datasource db {
    ///   provider   = "postgresql"
    ///   extensions = [citext(version: "2.1")]
    /// //             ^^^^^^^^^^^^^^^^^^^^^^^^ value
    /// //^^^^^^^^^^ key
    /// }
    /// ```
    pub fn push_custom_property(&mut self, key: &'a str, value: impl Into<Value<'a>>) {
        self.custom_properties.push((key, value.into()));
    }

    /// The documentation on top of the datasource.
    ///
    /// ```ignore
    /// /// This here is the documentation.
    /// datasource db {
    ///   provider = "postgresql"
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: impl Into<Cow<'a, str>>) {
        self.documentation = Some(Documentation(documentation.into()));
    }

    /// Create a rendering from a PSL datasource.
    pub fn from_psl(psl_ds: &'a psl::Datasource, force_namespaces: Option<&'a [String]>) -> Self {
        let namespaces: Vec<Text<_>> = match force_namespaces {
            Some(namespaces) => namespaces
                .iter()
                .map(AsRef::as_ref)
                .map(Cow::Borrowed)
                .map(Text)
                .collect(),
            None => psl_ds
                .namespaces
                .iter()
                .map(|(ns, _)| Text(Cow::Owned(ns.clone())))
                .collect(),
        };

        Self {
            name: &psl_ds.name,
            provider: Text(&psl_ds.provider),
            relation_mode: psl_ds.relation_mode,
            documentation: psl_ds.documentation.as_deref().map(Cow::Borrowed).map(Documentation),
            custom_properties: Default::default(),
            namespaces: Array::from(namespaces),
        }
    }
}

impl fmt::Display for Datasource<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref doc) = self.documentation {
            doc.fmt(f)?;
        }

        writeln!(f, "datasource {} {{", self.name)?;
        writeln!(f, "provider = {}", self.provider)?;

        if let Some(relation_mode) = self.relation_mode {
            writeln!(f, "relationMode = \"{relation_mode}\"")?;
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
    use crate::{configuration::*, value::*};
    use expect_test::expect;
    use psl::datamodel_connector::RelationMode;

    #[test]
    fn kitchen_sink() {
        let mut datasource = Datasource::new("db", "postgres");

        datasource.documentation("Here comes the sun king...\n\nEverybody's laughing,\nEverybody's happy!");
        datasource.relation_mode(RelationMode::ForeignKeys);

        let mut extensions = Array::new();
        extensions.push(Function::new("postgis"));
        {
            let mut ext = Function::new("uuid_ossp");
            ext.push_param(("map", Text::new("uuid-ossp")));
            extensions.push(ext);
        }

        datasource.push_custom_property("extensions", extensions);

        let expected = expect![[r#"
            /// Here comes the sun king...
            ///
            /// Everybody's laughing,
            /// Everybody's happy!
            datasource db {
              provider     = "postgres"
              relationMode = "foreignKeys"
              extensions   = [postgis, uuid_ossp(map: "uuid-ossp")]
            }
        "#]];

        let rendered = psl::reformat(&format!("{datasource}"), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
