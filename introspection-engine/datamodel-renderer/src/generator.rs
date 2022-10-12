use std::fmt;

use crate::{value::Array, Commented, Env, Text};

/// The generator block of the datasource.
#[derive(Debug)]
pub struct Generator<'a> {
    name: &'a str,
    provider: Env<'a>,
    output: Option<Env<'a>>,
    preview_features: Array<'a>,
    binary_targets: Array<'a>,
    documentation: Option<Commented<'a>>,
    config: Vec<(&'a str, Text<'a>)>,
}

impl<'a> Generator<'a> {
    /// A new generator with the required values set.
    pub fn new(name: &'a str, provider: Env<'a>) -> Self {
        Self {
            name,
            provider,
            output: None,
            preview_features: Array::default(),
            binary_targets: Array::default(),
            documentation: None,
            config: Vec::new(),
        }
    }

    /// Sets an output target.
    pub fn output(&mut self, output: Env<'a>) {
        self.output = Some(output);
    }

    /// Add a new preview feature to the generator block.
    pub fn push_preview_feature(&mut self, feature: &'a str) {
        self.preview_features.push(Text(feature));
    }

    /// Add a new binary target to the generator block.
    pub fn push_binary_target(&mut self, target: Env<'a>) {
        self.binary_targets.push(target)
    }

    /// Set the generator block documentation.
    pub fn documentation(&mut self, docs: &'a str) {
        self.documentation = Some(Commented::Documentation(docs));
    }

    /// Add a custom config value to the block.
    pub fn push_config_value(&mut self, key: &'a str, val: &'a str) {
        self.config.push((key, Text(val)));
    }
}

impl<'a> fmt::Display for Generator<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref doc) = self.documentation {
            doc.fmt(f)?;
        }

        writeln!(f, "generator {} {{", self.name)?;
        writeln!(f, "provider = {}", self.provider)?;

        if let Some(output) = self.output {
            writeln!(f, "output = {}", output)?;
        }

        if !self.preview_features.is_empty() {
            writeln!(f, "previewFeatures = {}", self.preview_features)?;
        }

        if !self.binary_targets.is_empty() {
            writeln!(f, "binaryTargets = {}", self.binary_targets)?;
        }

        for (k, v) in self.config.iter() {
            writeln!(f, "{k} = {v}")?;
        }

        f.write_str("}\n")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use expect_test::expect;

    #[test]
    fn kitchen_sink() {
        let mut generator = Generator::new("client", Env::value("prisma-client-js"));

        generator.documentation("Here comes the sun.\n\nAnd I say,\nIt's alright.");
        generator.output(Env::value("/dev/null"));

        generator.push_preview_feature("multiSchema");
        generator.push_preview_feature("postgresExtensions");
        generator.push_binary_target(Env::variable("BINARY TARGET"));
        generator.push_config_value("customValue", "meow");
        generator.push_config_value("otherValue", "purr");

        let expected = expect![[r#"
            /// Here comes the sun.
            ///
            /// And I say,
            /// It's alright.
            generator client {
              provider        = "prisma-client-js"
              output          = "/dev/null"
              previewFeatures = ["multiSchema", "postgresExtensions"]
              binaryTargets   = [env("BINARY TARGET")]
              customValue     = "meow"
              otherValue      = "purr"
            }
        "#]];

        let rendered = psl::reformat(&format!("{generator}"), 2).unwrap();
        expected.assert_eq(&rendered)
    }
}
