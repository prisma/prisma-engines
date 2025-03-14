use crate::value::{Array, Documentation, Env, Text, Value};
use itertools::Itertools;
use psl::PreviewFeature;
use std::{borrow::Cow, fmt};

/// The generator block of the datasource.
#[derive(Debug)]
pub struct Generator<'a> {
    name: &'a str,
    provider: Env<'a>,
    output: Option<Env<'a>>,
    preview_features: Option<Array<Text<PreviewFeature>>>,
    binary_targets: Array<Env<'a>>,
    documentation: Option<Documentation<'a>>,
    config: Vec<(&'a str, Value<'a>)>,
}

impl<'a> Generator<'a> {
    /// A new generator with the required values set.
    ///
    /// ```ignore
    /// generator js {
    /// //        ^^ name
    ///   provider = "prisma-client-js"
    /// //            ^^^^^^^^^^^^^^^^ provider
    /// }
    /// ```
    pub fn new(name: &'a str, provider: impl Into<Env<'a>>) -> Self {
        Self {
            name,
            provider: provider.into(),
            output: None,
            preview_features: None,
            binary_targets: Array::new(),
            documentation: None,
            config: Vec::new(),
        }
    }

    /// Sets an output target.
    ///
    /// ```ignore
    /// generator js {
    ///   output = env("OUTPUT_DIR")
    /// //              ^^^^^^^^^^ this
    /// }
    /// ```
    pub fn output(&mut self, output: impl Into<Env<'a>>) {
        self.output = Some(output.into());
    }

    /// Add a new preview feature to the generator block.
    ///
    /// ```ignore
    /// generator js {
    ///   previewFeatures = ["postgresqlExtensions"]
    /// //                    ^^^^^^^^^^^^^^^^^^^^ pushed here
    /// }
    /// ```
    pub fn push_preview_feature(&mut self, feature: PreviewFeature) {
        let features = self.preview_features.get_or_insert_with(Array::new);
        features.push(Text(feature));
    }

    /// Add a new binary target to the generator block.
    ///
    /// ```ignore
    /// generator js {
    ///   binaryTargets = [env("FOO_TARGET")]
    /// //                 ^^^^^^^^^^^^^^^^^ pushed here
    /// }
    /// ```
    pub fn push_binary_target(&mut self, target: impl Into<Env<'a>>) {
        self.binary_targets.push(target.into())
    }

    /// Set the generator block documentation.
    ///
    /// ```ignore
    /// /// This here is the documentation.
    /// generator js {
    ///   provider = "prisma-client-js"
    /// }
    /// ```
    pub fn documentation(&mut self, docs: impl Into<Cow<'a, str>>) {
        self.documentation = Some(Documentation(docs.into()));
    }

    /// Add a custom config value to the block.
    ///
    /// ```ignore
    /// generator js {
    ///   provider = "prisma-client-js"
    ///   custom   = "foo"
    /// //           ^^^^^ value
    /// //^^^^^^ key
    /// }
    /// ```
    pub fn push_config_value(&mut self, key: &'a str, val: impl Into<Value<'a>>) {
        self.config.push((key, val.into()));
    }

    /// Create a rendering from a PSL generator.
    pub fn from_psl(psl_gen: &'a psl::Generator) -> Self {
        let preview_features = psl_gen
            .preview_features
            .map(|f| f.iter().map(Text).collect::<Vec<Text<_>>>())
            .map(Array::from);

        let binary_targets: Vec<Env<'_>> = psl_gen.binary_targets.iter().map(Env::from).collect();

        let config = psl_gen.config.iter().map(|(k, v)| (k.as_str(), v.into())).collect();

        Self {
            name: &psl_gen.name,
            provider: Env::from(&psl_gen.provider),
            output: psl_gen.output.as_ref().map(Env::from),
            preview_features,
            binary_targets: Array::from(binary_targets),
            documentation: psl_gen.documentation.as_deref().map(Cow::Borrowed).map(Documentation),
            config,
        }
    }
}

impl fmt::Display for Generator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref doc) = self.documentation {
            doc.fmt(f)?;
        }

        writeln!(f, "generator {} {{", self.name)?;
        writeln!(f, "provider = {}", self.provider)?;

        if let Some(output) = self.output {
            writeln!(f, "output = {output}")?;
        }

        if let Some(ref features) = self.preview_features {
            writeln!(f, "previewFeatures = {features}")?;
        }

        if !self.binary_targets.is_empty() {
            writeln!(f, "binaryTargets = {}", self.binary_targets)?;
        }

        for (k, v) in self.config.iter().sorted_by_key(|(k, _)| k) {
            writeln!(f, "{k} = {v}")?;
        }

        f.write_str("}\n")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{configuration::*, value::*};
    use expect_test::expect;
    use psl::PreviewFeature;

    #[test]
    fn kitchen_sink() {
        let mut generator = Generator::new("client", Env::value("prisma-client-js"));

        generator.documentation("Here comes the sun.\n\nAnd I say,\nIt's alright.");

        generator.output(Env::value("/dev/null"));
        generator.push_binary_target(Env::variable("BINARY TARGET"));

        generator.push_preview_feature(PreviewFeature::MultiSchema);
        generator.push_preview_feature(PreviewFeature::PostgresqlExtensions);

        generator.push_config_value("customValue", "meow");
        generator.push_config_value("otherValue", "purr");

        generator.push_config_value("customFeatures", vec![Value::from("enums"), Value::from("models")]);
        generator.push_config_value(
            "afterGenerate",
            vec![
                Value::from("lambda"),
                Vec::<Value>::new().into(),
                vec![
                    Value::from("print"),
                    vec![Value::from("quote"), Value::from("done!")].into(),
                ]
                .into(),
            ],
        );

        generator.push_config_value("customEnvValue", Env::variable("var"));

        let expected = expect![[r#"
            /// Here comes the sun.
            ///
            /// And I say,
            /// It's alright.
            generator client {
              provider        = "prisma-client-js"
              output          = "/dev/null"
              previewFeatures = ["multiSchema", "postgresqlExtensions"]
              binaryTargets   = [env("BINARY TARGET")]
              afterGenerate   = ["lambda", [], ["print", ["quote", "done!"]]]
              customEnvValue  = env("var")
              customFeatures  = ["enums", "models"]
              customValue     = "meow"
              otherValue      = "purr"
            }
        "#]];

        let rendered = psl::reformat(&format!("{generator}"), 2).unwrap();
        expected.assert_eq(&rendered)
    }

    #[test]
    fn creates_consistent_ordering() {
        let mut generator1 = Generator::new("client", Env::value("prisma-client-js"));
        generator1.push_config_value("first", "A");
        generator1.push_config_value("second", "B");
        let rendered1 = psl::reformat(&format!("{generator1}"), 2).unwrap();

        let mut generator2 = Generator::new("client", Env::value("prisma-client-js"));
        generator2.push_config_value("second", "B");
        generator2.push_config_value("first", "A");
        let rendered2 = psl::reformat(&format!("{generator2}"), 2).unwrap();

        let expected = expect![[r#"
            generator client {
              provider = "prisma-client-js"
              first    = "A"
              second   = "B"
            }
        "#]];

        expected.assert_eq(&rendered1);
        expected.assert_eq(&rendered2)
    }
}
