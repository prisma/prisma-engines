use crate::ast;
use crate::configuration::Datasource;

pub struct DatasourceSerializer {}

impl DatasourceSerializer {
    pub fn add_sources_to_ast(sources: &[Datasource], ast_datamodel: &mut ast::SchemaAst) {
        let mut tops: Vec<ast::Top> = Vec::with_capacity(ast_datamodel.tops.len() + sources.len());

        for source in sources {
            tops.push(ast::Top::Source(Self::lower_datasource(source)))
        }

        // Prepend sources.
        tops.append(&mut ast_datamodel.tops);

        ast_datamodel.tops = tops;
    }

    fn lower_datasource(source: &Datasource) -> ast::SourceConfig {
        let mut arguments: Vec<ast::Argument> = vec![ast::Argument::new_string("provider", &source.active_provider)];

        arguments.push(super::lower_string_from_env_var("url", &source.url));
        if let Some((shadow_database_url, _)) = &source.shadow_database_url {
            arguments.push(super::lower_string_from_env_var(
                "shadowDatabaseUrl",
                shadow_database_url,
            ))
        }

        ast::SourceConfig {
            name: ast::Identifier::new(&source.name),
            properties: arguments,
            documentation: source.documentation.clone().map(|text| ast::Comment { text }),
            span: ast::Span::empty(),
        }
    }
}
