//! Types related to the _configuration section_ in the PSL.
//!
//! Includes the `datasource` and `generator` definitions.

mod datasource;
mod generator;

pub use datasource::Datasource;
pub use generator::Generator;
use psl::ValidatedSchema;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

/// The configuration part of a data model. First the generators, then
/// the datasources.
#[derive(Debug, Default)]
pub struct Configuration<'a> {
    /// Generators blocks by file name.
    pub generators: HashMap<Cow<'a, str>, Vec<Generator<'a>>>,
    /// Datasources blocks by file name.
    pub datasources: HashMap<Cow<'a, str>, Vec<Datasource<'a>>>,
}

impl<'a> Configuration<'a> {
    /// Add a new generator to the configuration.
    pub fn push_generator(&mut self, file: impl Into<Cow<'a, str>>, generator: Generator<'a>) {
        self.generators.entry(file.into()).or_default().push(generator);
    }

    /// Add a new datasource to the configuration.
    pub fn push_datasource(&mut self, file: impl Into<Cow<'a, str>>, datasource: Datasource<'a>) {
        self.datasources.entry(file.into()).or_default().push(datasource);
    }

    /// Create a rendering from a PSL datasource.
    pub fn from_psl(
        psl_cfg: &'a psl::Configuration,
        prev_schema: &'a ValidatedSchema,
        force_namespaces: Option<&'a [String]>,
    ) -> Self {
        let mut config = Self::default();

        for generator in &psl_cfg.generators {
            let file_name = prev_schema.db.file_name(generator.span.file_id);

            config.push_generator(Cow::Borrowed(file_name), Generator::from_psl(generator));
        }

        for datasource in &psl_cfg.datasources {
            let file_name = prev_schema.db.file_name(datasource.span.file_id);

            config.push_datasource(
                Cow::Borrowed(file_name),
                Datasource::from_psl(datasource, force_namespaces),
            );
        }

        config
    }
}

impl<'a> fmt::Display for Configuration<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (_, generators) in self.generators.iter() {
            for generator in generators {
                generator.fmt(f)?
            }
        }

        for (_, datasources) in self.datasources.iter() {
            for datasource in datasources {
                datasource.fmt(f)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{configuration::*, value::*};
    use expect_test::expect;

    #[test]
    fn minimal() {
        let mut config = Configuration::default();
        let file_name = "schema.prisma";

        config.push_generator(
            file_name.to_owned(),
            Generator::new("client", Env::value("prisma-client-js")),
        );
        config.push_datasource(
            file_name.to_owned(),
            Datasource::new("db", "postgres", Env::variable("DATABASE_URL")),
        );

        let rendered = psl::reformat(&format!("{config}"), 2).unwrap();

        let expected = expect![[r#"
            generator client {
              provider = "prisma-client-js"
            }

            datasource db {
              provider = "postgres"
              url      = env("DATABASE_URL")
            }
        "#]];

        expected.assert_eq(&rendered);
    }

    #[test]
    fn not_so_minimal() {
        let mut config = Configuration::default();
        let file_name = "schema.prisma";

        config.push_generator(
            file_name.to_owned(),
            Generator::new("js", Env::value("prisma-client-js")),
        );
        config.push_generator(
            file_name.to_owned(),
            Generator::new("go", Env::value("prisma-client-go")),
        );
        config.push_datasource(
            file_name.to_owned(),
            Datasource::new("pg", "postgres", Env::variable("PG_DATABASE_URL")),
        );
        config.push_datasource(
            file_name.to_owned(),
            Datasource::new("my", "mysql", Env::variable("MY_DATABASE_URL")),
        );

        let expected = expect![[r#"
            generator js {
              provider = "prisma-client-js"
            }

            generator go {
              provider = "prisma-client-go"
            }

            datasource pg {
              provider = "postgres"
              url      = env("PG_DATABASE_URL")
            }

            datasource my {
              provider = "mysql"
              url      = env("MY_DATABASE_URL")
            }
        "#]];

        let rendered = psl::reformat(&format!("{config}"), 2).unwrap();
        expected.assert_eq(&rendered);
    }
}
