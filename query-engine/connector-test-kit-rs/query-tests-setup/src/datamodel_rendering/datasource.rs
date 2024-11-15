use std::iter;

use indexmap::IndexMap;
use itertools::Itertools;

pub struct DatasourceBuilder<'a> {
    name: &'a str,
    properties: IndexMap<&'static str, String>,
}

impl<'a> DatasourceBuilder<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            properties: Default::default(),
        }
    }

    pub fn provider(mut self, provider: impl AsRef<str>) -> Self {
        self.add_debug("provider", provider.as_ref());
        self
    }

    pub fn url(mut self, url: impl AsRef<str>) -> Self {
        self.add_debug("url", url.as_ref());
        self
    }

    pub fn relation_mode(mut self, relation_mode: impl AsRef<str>) -> Self {
        self.add_debug("relationMode", relation_mode.as_ref());
        self
    }

    pub fn schemas(mut self, schemas: &[&str]) -> Self {
        self.add_debug("schemas", schemas);
        self
    }

    pub fn schemas_if_not_empty(self, schemas: &[&str]) -> Self {
        if schemas.is_empty() {
            self
        } else {
            self.schemas(schemas)
        }
    }

    pub fn extensions(mut self, extensions: &[&str]) -> Self {
        self.properties
            .insert("extensions", format!("[{}]", extensions.iter().join(", ")));
        self
    }

    pub fn extensions_if_not_empty(self, extensions: &[&str]) -> Self {
        if extensions.is_empty() {
            self
        } else {
            self.extensions(extensions)
        }
    }

    pub fn render(self) -> String {
        iter::once(format!("datasource {} {{", self.name))
            .chain(self.properties.into_iter().map(|(k, v)| format!("    {k} = {v}")))
            .chain(iter::once("}\n".into()))
            .join("\n")
    }

    fn add_debug(&mut self, key: &'static str, value: impl std::fmt::Debug) {
        self.properties.insert(key, format!("{:?}", value));
    }
}

#[cfg(test)]
mod test {
    use indoc::indoc;

    use super::DatasourceBuilder;

    #[test]
    fn all() {
        let datasource = DatasourceBuilder::new("test")
            .provider("postgresql")
            .url("postgres://test")
            .relation_mode("foreignKeys")
            .schemas(&["public"])
            .extensions(&["citext", r#"postgis(version: "2.1")"#])
            .render();

        assert_eq!(
            datasource,
            indoc! {
                r#"
                datasource test {
                    provider = "postgresql"
                    url = "postgres://test"
                    relationMode = "foreignKeys"
                    schemas = ["public"]
                    extensions = [citext, postgis(version: "2.1")]
                }
                "#
            }
        )
    }

    #[test]
    fn partial_mixed() {
        let datasource = DatasourceBuilder::new("db")
            .url("mysql://test")
            .provider("mysql")
            .render();

        assert_eq!(
            datasource,
            indoc! {
                r#"
                datasource db {
                    url = "mysql://test"
                    provider = "mysql"
                }
                "#
            }
        )
    }

    #[test]
    fn skip_empty_arrays() {
        let datasource = DatasourceBuilder::new("invalid")
            .schemas_if_not_empty(&[])
            .extensions_if_not_empty(&[])
            .render();

        assert_eq!(
            datasource,
            indoc! {
                r#"
                datasource invalid {
                }
                "#
            }
        )
    }
}
