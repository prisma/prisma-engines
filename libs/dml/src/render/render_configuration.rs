use psl_core::{Configuration, Datasource, Generator, StringFromEnvVar};
use schema_ast::string_literal;
use std::fmt::{self, Write as _};

pub(crate) fn render_configuration(config: &Configuration, out: &mut String) {
    for generator in &config.generators {
        render_generator(generator, out).unwrap();
    }

    for source in &config.datasources {
        render_datasource(source, out).unwrap();
    }
}

fn render_datasource(datasource: &Datasource, out: &mut String) -> fmt::Result {
    if let Some(docs) = &datasource.documentation {
        super::render_documentation(docs, false, out);
    }
    writeln!(
        out,
        "datasource {} {{\nprovider = {}",
        datasource.name,
        string_literal(datasource.active_provider)
    )?;

    out.push_str("url = ");
    render_string_from_env(&datasource.url, out)?;
    out.push('\n');

    if let Some((shadow_database_url, _)) = &datasource.shadow_database_url {
        out.push_str("shadowDatabaseUrl = ");
        render_string_from_env(shadow_database_url, out)?;
        out.push('\n');
    }

    if let Some(relation_mode) = datasource.relation_mode {
        writeln!(out, "relationMode = {}", string_literal(&relation_mode.to_string()))?;
    }

    out.write_str("}\n")
}

fn render_generator(generator: &Generator, out: &mut String) -> fmt::Result {
    if let Some(docs) = &generator.documentation {
        super::render_documentation(docs, false, out);
    }
    out.push_str("generator ");
    out.push_str(&generator.name);
    out.push_str(" {\n");

    // Provider
    {
        out.push_str("provider = \"");
        out.push_str(generator.provider.as_literal().unwrap());
        out.push_str("\"\n");
    }

    if let Some(output) = &generator.output {
        out.push_str("output = ");
        render_string_from_env(output, out)?;
        out.push('\n');
    }

    if let Some(ref features) = &generator.preview_features {
        let mut feats = features.iter().peekable();
        out.push_str("previewFeatures = [");
        while let Some(feature) = feats.next() {
            write!(out, "{}", string_literal(&feature.to_string()))?;
            if feats.peek().is_some() {
                out.push(',');
            }
        }
        out.push_str("]\n");
    }

    if !generator.binary_targets.is_empty() {
        let mut targets = generator.binary_targets.iter().peekable();
        out.push_str("binaryTargets = [");
        while let Some(target) = targets.next() {
            render_string_from_env(target, out)?;
            if targets.peek().is_some() {
                out.push(',');
            }
        }
        out.push_str("]\n");
    }

    for (key, value) in &generator.config {
        writeln!(out, "{key} = {}", string_literal(value))?;
    }
    out.write_str("}\n")
}

fn render_string_from_env(string_from_env: &StringFromEnvVar, out: &mut String) -> fmt::Result {
    match &string_from_env.from_env_var {
        Some(var_name) => out.write_fmt(format_args!("env({})", string_literal(var_name))),
        None => out.write_fmt(format_args!(
            "{}",
            string_literal(string_from_env.value.as_ref().unwrap())
        )),
    }
}
