use crate::{calculate_datamodel::OutputContext, defaults, pair::ScalarFieldPair};
use datamodel_renderer::datamodel as renderer;
use sql_schema_describer::ColumnArity;

pub(crate) fn render<'a>(field: ScalarFieldPair<'a>, output: &mut OutputContext<'a>) -> renderer::ModelField<'a> {
    let mut rendered = renderer::ModelField::new(field.name(), field.prisma_type());

    match field.arity() {
        ColumnArity::Nullable => rendered.optional(),
        ColumnArity::List => rendered.array(),
        ColumnArity::Required => (),
    }

    if field.is_unsupported() {
        rendered.unsupported();
    }

    if let Some(map) = field.mapped_name() {
        rendered.map(map);
    }

    if let Some((prefix, r#type, params)) = field.native_type() {
        rendered.native_type(prefix, r#type, params)
    }

    if let Some(docs) = field.documentation() {
        rendered.documentation(docs);
    }

    if let Some(default) = defaults::render(field, output) {
        rendered.default(default);
    }

    if field.is_updated_at() {
        rendered.updated_at();
    }

    if field.is_ignored() {
        rendered.ignore();
    }

    if let Some(pk) = field.id() {
        let field = pk.field().unwrap();
        let mut definition = renderer::IdFieldDefinition::default();

        if let Some(clustered) = pk.clustered() {
            definition.clustered(clustered);
        }

        if let Some(sort_order) = field.sort_order() {
            definition.sort_order(sort_order);
        }

        if let Some(length) = field.length() {
            definition.length(length);
        }

        if let Some(map) = pk.mapped_name() {
            definition.map(map);
        }

        rendered.id(definition);
    }

    if let Some(unique) = field.unique() {
        let mut opts = renderer::IndexFieldOptions::default();
        let field = unique.field().unwrap();

        if let Some(map) = unique.mapped_name() {
            opts.map(map);
        }

        if let Some(clustered) = unique.clustered() {
            opts.clustered(clustered);
        }

        if let Some(sort_order) = field.sort_order() {
            opts.sort_order(sort_order);
        }

        if let Some(length) = field.length() {
            opts.length(length);
        }

        rendered.unique(opts);
    }

    if field.remapped_name_empty() {
        let docs = "This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*";
        rendered.documentation(docs);
        rendered.commented_out();
    }

    if field.remapped_name_from_psl() {
        let mf = crate::warnings::ModelAndField {
            model: field.model().name().to_string(),
            field: field.name().to_string(),
        };

        output.warnings.remapped_fields.push(mf);
    }

    if field.is_unsupported() {
        let mf = crate::warnings::ModelAndFieldAndType {
            model: field.model().name().to_string(),
            field: field.name().to_string(),
            tpe: field.prisma_type().to_string(),
        };

        output.warnings.unsupported_types.push(mf)
    }

    rendered
}
