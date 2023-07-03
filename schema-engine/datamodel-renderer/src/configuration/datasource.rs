use crate::value::{Array, Documentation, Env, Text, Value};
use core::fmt;
use psl::datamodel_connector::RelationMode;
use std::{borrow::Cow, default::Default};

/// The datasource block in a PSL file.
#[derive(Debug)]
pub struct Datasource<'a> {
    name: &'a str,
    provider: Text<&'a str>,
    url: Env<'a>,
    direct_url: Option<Env<'a>>,
    shadow_database_url: Option<Env<'a>>,
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
    ///   url      = env("DATABASE_URL")
    /// //                ^^^^^^^^^^^^ url
    /// }
    /// ```
    pub fn new(name: &'a str, provider: &'a str, url: impl Into<Env<'a>>) -> Self {
        Self {
            name,
            provider: Text(provider),
            url: url.into(),
            direct_url: None,
            shadow_database_url: None,
            relation_mode: None,
            custom_properties: Default::default(),
            documentation: None,
            namespaces: Array::new(),
        }
    }

    /// Used for schema engine to reflect the contents of migrations directory
    /// to diff against the actual database.
    ///
    /// ```ignore
    /// datasource db {
    ///   provider          = "postgresql"
    ///   url               = env("DATABASE_URL")
    ///   shadowDatabaseUrl = env("SHADOW_DATABASE_URL")
    /// //                         ^^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn shadow_database_url(&mut self, url: impl Into<Env<'a>>) {
        self.shadow_database_url = Some(url.into());
    }

    /// Who handles referential integrity.
    ///
    /// ```ignore
    /// datasource db {
    ///   provider     = "postgresql"
    ///   url          = env("DATABASE_URL")
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
    ///   url          = env("DATABASE_URL")
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
    ///   url      = env("DATABASE_URL")
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: impl Into<Cow<'a, str>>) {
        self.documentation = Some(Documentation(documentation.into()));
    }

    /// Create a rendering from a PSL datasource.
    pub fn from_psl(psl_ds: &'a psl::Datasource, force_namespaces: Option<&'a [String]>) -> Self {
        let shadow_database_url = psl_ds.shadow_database_url.as_ref().map(|(url, _)| Env::from(url));

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
            url: Env::from(&psl_ds.url),
            direct_url: psl_ds.direct_url.as_ref().map(Env::from),
            shadow_database_url,
            relation_mode: psl_ds.relation_mode,
            documentation: psl_ds.documentation.as_deref().map(Cow::Borrowed).map(Documentation),
            custom_properties: Default::default(),
            namespaces: Array::from(namespaces),
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
        if let Some(direct_url) = self.direct_url {
            writeln!(f, "directUrl = {direct_url}")?;
        }

        if let Some(url) = self.shadow_database_url {
            writeln!(f, "shadowDatabaseUrl = {url}")?;
        }

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
        let mut datasource = Datasource::new("db", "postgres", Env::variable("DATABASE_URL"));

        datasource.documentation("Here comes the sun king...\n\nEverybody's laughing,\nEverybody's happy!");
        datasource.shadow_database_url(Env::variable("SHADOW_DATABASE_URL"));
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
              provider          = "postgres"
              url               = env("DATABASE_URL")
              shadowDatabaseUrl = env("SHADOW_DATABASE_URL")
              relationMode      = "foreignKeys"
              extensions        = [postgis, uuid_ossp(map: "uuid-ossp")]
            }
        "#]];

        let rendered = psl::reformat(&format!("{datasource}"), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
