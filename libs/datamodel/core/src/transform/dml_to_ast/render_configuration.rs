use crate::{transform::dml_to_ast::render_string_from_env, Configuration, Datasource, Generator};
use schema_ast::renderer::Renderer;

pub(crate) fn render_configuration(config: &Configuration, out: &mut String) {
    for generator in &config.generators {
        render_generator(generator, out);
    }

    for source in &config.datasources {
        render_datasource(source, out)
    }
}

fn render_datasource(datasource: &Datasource, out: &mut String) {
    if let Some(docs) = &datasource.documentation {
        for line in docs.lines() {
            out.push_str("/// ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("datasource ");
    out.push_str(&datasource.name);
    out.push_str(" {\n");

    {
        out.push_str("provider = ");
        Renderer::render_str(out, &datasource.active_provider);
        out.push('\n');
    }

    {
        out.push_str("url = ");
        render_string_from_env(&datasource.url, out);
        out.push('\n');
    }

    if let Some((shadow_database_url, _)) = &datasource.shadow_database_url {
        out.push_str("shadowDatabaseUrl = ");
        render_string_from_env(shadow_database_url, out);
        out.push('\n');
    }

    if let Some(referential_integrity) = datasource.referential_integrity {
        out.push_str("referentialIntegrity = ");
        Renderer::render_str(out, &referential_integrity.to_string());
        out.push('\n');
    }

    out.push_str("}\n");
}

fn render_generator(generator: &Generator, out: &mut String) {
    if let Some(docs) = &generator.documentation {
        for line in docs.lines() {
            out.push_str("/// ");
            out.push_str(line);
            out.push('\n');
        }
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
        super::render_string_from_env(output, out);
        out.push('\n');
    }

    if let Some(ref features) = &generator.preview_features {
        let mut feats = features.iter().peekable();
        out.push_str("previewFeatures = [");
        while let Some(feature) = feats.next() {
            Renderer::render_str(out, &feature.to_string());
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
            render_string_from_env(target, out);
            if targets.peek().is_some() {
                out.push(',');
            }
        }
        out.push_str("]\n");
    }

    for (key, value) in &generator.config {
        out.push_str(key);
        out.push_str(" = ");
        Renderer::render_str(out, value);
        out.push('\n');
    }
    out.push_str("}\n");
}
