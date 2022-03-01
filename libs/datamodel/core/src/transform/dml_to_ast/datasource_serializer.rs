use crate::{ast, Configuration, PreviewFeature};

pub fn add_sources_to_ast(config: &Configuration, ast_datamodel: &mut ast::SchemaAst) {
    let mut tops: Vec<ast::Top> = Vec::with_capacity(ast_datamodel.tops.len() + config.datasources.len());
    let preview_features = config.preview_features();

    for source in config.datasources.iter() {
        let mut arguments: Vec<ast::ConfigBlockProperty> = vec![ast::ConfigBlockProperty {
            name: ast::Identifier::new("provider"),
            value: ast::Expression::StringValue(source.active_provider.clone(), ast::Span::empty()),
            span: ast::Span::empty(),
        }];

        arguments.push(super::lower_string_from_env_var("url", &source.url));
        if let Some((shadow_database_url, _)) = &source.shadow_database_url {
            arguments.push(super::lower_string_from_env_var(
                "shadowDatabaseUrl",
                shadow_database_url,
            ))
        }

        if preview_features.contains(PreviewFeature::ReferentialIntegrity) {
            if let Some(referential_integrity) = source.referential_integrity {
                let property = ast::ConfigBlockProperty {
                    name: ast::Identifier::new("referentialIntegrity"),
                    value: ast::Expression::StringValue(referential_integrity.to_string(), ast::Span::empty()),
                    span: ast::Span::empty(),
                };
                arguments.push(property);
            }
        }

        tops.push(ast::Top::Source(ast::SourceConfig {
            name: ast::Identifier::new(&source.name),
            properties: arguments,
            documentation: source.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        }))
    }

    // Prepend sources.
    tops.append(&mut ast_datamodel.tops);

    ast_datamodel.tops = tops;
}
