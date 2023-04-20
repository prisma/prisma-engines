//! Types related to the _configuration section_ in the PSL.
//!
//! Includes the `datasource` and `generator` definitions.

mod datasource;
mod generator;

pub use datasource::Datasource;
pub use generator::Generator;

use std::fmt;

/// The configuration part of a data model. First the generators, then
/// the datasources.
#[derive(Debug, Default)]
pub struct Configuration<'a> {
    generators: Vec<Generator<'a>>,
    datasources: Vec<Datasource<'a>>,
}

impl<'a> Configuration<'a> {
    /// Add a new generator to the configuration.
    pub fn push_generator(&mut self, generator: Generator<'a>) {
        self.generators.push(generator);
    }

    /// Add a new datasource to the configuration.
    pub fn push_datasource(&mut self, datasource: Datasource<'a>) {
        self.datasources.push(datasource);
    }

    /// Create a rendering from a PSL datasource.
    pub fn from_psl(psl_cfg: &'a psl::Configuration, force_namespaces: Option<&'a [String]>) -> Self {
        let mut config = Self::default();

        for generator in psl_cfg.generators.iter() {
            config.push_generator(Generator::from_psl(generator));
        }

        for datasource in psl_cfg.datasources.iter() {
            config.push_datasource(Datasource::from_psl(datasource, force_namespaces));
        }

        config
    }
}

impl<'a> fmt::Display for Configuration<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for generator in self.generators.iter() {
            generator.fmt(f)?
        }

        for datasource in self.datasources.iter() {
            datasource.fmt(f)?;
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

        config.push_generator(Generator::new("client", Env::value("prisma-client-js")));
        config.push_datasource(Datasource::new("db", "postgres", Env::variable("DATABASE_URL")));

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

        config.push_generator(Generator::new("js", Env::value("prisma-client-js")));
        config.push_generator(Generator::new("go", Env::value("prisma-client-go")));
        config.push_datasource(Datasource::new("pg", "postgres", Env::variable("PG_DATABASE_URL")));
        config.push_datasource(Datasource::new("my", "mysql", Env::variable("MY_DATABASE_URL")));

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
