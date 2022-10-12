use crate::*;
use psl_core::datamodel_connector::{constraint_names::ConstraintNames, Connector, EmptyDatamodelConnector};
use psl_core::{parser_database as db, Datasource, RelationNames};
use schema_ast::string_literal;
use std::fmt::Write;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RenderParams<'a> {
    pub datasource: Option<&'a Datasource>,
    pub datamodel: &'a Datamodel,
}

impl RenderParams<'_> {
    fn connector(&self) -> &'static dyn Connector {
        self.datasource
            .map(|ds| ds.active_connector)
            .unwrap_or(&EmptyDatamodelConnector)
    }
}

pub(crate) fn render_datamodel(params: RenderParams<'_>, out: &mut String) {
    for r#type in &params.datamodel.composite_types {
        render_composite_type(r#type, params, out);
    }

    for model in params.datamodel.models().filter(|m| !m.is_generated) {
        render_model(model, params, out);
    }

    for enm in params.datamodel.enums() {
        render_enum(enm, out)
    }
}

fn render_enum(enm: &Enum, out: &mut String) {
    if let Some(docs) = &enm.documentation {
        super::render_documentation(docs, false, out);
    }
    out.push_str("enum ");
    out.push_str(&enm.name);
    out.push_str("{\n");

    for variant in enm.values.iter() {
        if let Some(docs) = &variant.documentation {
            super::render_documentation(docs, false, out);
        }

        if variant.commented_out {
            out.push_str("// ");
        }

        out.push_str(&variant.name);

        if let Some(mapped_name) = &variant.database_name {
            render_map_attribute(mapped_name, "@", out);
        }

        out.push('\n');
    }

    if let Some(mapped_name) = &enm.database_name {
        render_map_attribute(mapped_name, "@@", out);
        out.push('\n');
    }

    out.push_str("}\n");
}

fn render_map_attribute(mapped_name: &str, owl: &str, out: &mut String) {
    write!(out, " {owl}map({})", string_literal(mapped_name)).unwrap()
}

fn render_model(model: &Model, params: RenderParams<'_>, out: &mut String) {
    if let Some(docs) = &model.documentation {
        super::render_documentation(docs, model.is_commented_out, out);
    }

    if model.is_commented_out {
        let mut commented_out = String::new();
        render_model_impl(model, params, &mut commented_out);
        for line in commented_out.lines() {
            out.push_str("// ");
            out.push_str(line);
            out.push('\n');
        }
    } else {
        render_model_impl(model, params, out);
    }
}

fn render_model_impl(model: &Model, params: RenderParams<'_>, out: &mut String) {
    out.push_str("model ");
    out.push_str(&model.name);
    out.push_str(" {\n");

    for field in &model.fields {
        if let Some(docs) = &field.documentation() {
            super::render_documentation(docs, field.is_commented_out(), out);
        }

        if field.is_commented_out() {
            out.push_str("// ");
        }

        out.push_str(field.name());
        out.push(' ');
        render_field_type(&field.field_type(), out);
        render_field_arity(field.arity(), out);
        render_field_attributes(field, model, params, out);
        out.push('\n');
    }

    out.push('\n');
    render_model_attributes(model, params, out);
    out.push_str("}\n");
}

fn render_composite_type(tpe: &CompositeType, params: RenderParams<'_>, out: &mut String) {
    out.push_str("type ");
    out.push_str(&tpe.name);
    out.push_str("{\n");

    for field in &tpe.fields {
        if let Some(docs) = &field.documentation {
            super::render_documentation(docs, field.is_commented_out, out);
        }

        if field.is_commented_out {
            out.push_str("// ");
        }

        out.push_str(&field.name);
        out.push(' ');
        render_composite_field_type(&field.r#type, out);
        render_field_arity(&field.arity, out);

        // @map
        if let Some(mapped_name) = &field.database_name {
            render_map_attribute(mapped_name, "@", out);
        }

        if let (Some((scalar_type, native_type)), Some(datasource)) = (field.r#type.as_native_type(), params.datasource)
        {
            render_native_type_attribute(scalar_type, native_type, datasource, out);
        }

        out.push('\n');
    }

    out.push_str("}\n");
}

fn render_field_attributes(field: &Field, model: &Model, params: RenderParams<'_>, out: &mut String) {
    // @updatedAt
    if field.is_updated_at() {
        out.push_str(" @updatedAt");
    }

    // @unique
    let field_unique = model
        .indices
        .iter()
        .filter(|idx| idx.is_unique() && idx.defined_on_field)
        .find(|idx| idx.fields.len() == 1 && idx.fields[0].path[0].0.as_str() == field.name());
    if let Some(index_def) = field_unique {
        let attr = match index_def.tpe {
            IndexType::Unique => " @unique",
            _ => unreachable!(),
        };
        out.push_str(attr);
        let field = &index_def.fields[0];
        let mut args = Vec::new();
        if let Some(length) = field.length {
            args.push((Some("length"), length.to_string()));
        }

        if let Some(SortOrder::Desc) = field.sort_order {
            args.push((Some("sort"), "Desc".to_string()));
        }

        push_index_arguments(index_def, model, params, &mut args);
        render_arguments(&mut args, out);
    }

    // @id
    if let Field::ScalarField(sf) = field {
        if model.field_is_primary_and_defined_on_field(&sf.name) {
            let mut args = Vec::new();
            let pk = model.primary_key.as_ref().unwrap();

            out.push_str(" @id");

            if let Some(name) = &pk.name {
                args.push((Some("name"), string_literal(name).to_string()));
            }

            if let Some(db_name) = pk.db_name.as_ref().filter(|s| !s.is_empty()) {
                if !primary_key_name_matches(pk, model, params.connector()) {
                    args.push((Some("map"), string_literal(db_name).to_string()));
                }
            }

            if let Some(length) = pk.fields.first().unwrap().length {
                args.push((Some("length"), length.to_string()));
            }

            if let Some(SortOrder::Desc) = pk.fields.first().unwrap().sort_order {
                args.push((Some("sort"), "Desc".to_string()));
            }

            if matches!(pk.clustered, Some(false)) {
                args.push((Some("clustered"), "false".to_string()));
            }

            render_arguments(&mut args, out);
        }
    }

    // @default
    if let Some(default_value) = field.default_value() {
        out.push_str(" @default(");
        render_default_value(default_value, out);

        let prisma_default = ConstraintNames::default_name(model.name(), field.name(), params.connector());
        if let Some(name) = default_value.db_name() {
            if name != prisma_default {
                out.write_fmt(format_args!(", map: {}", string_literal(name))).unwrap();
            }
        }

        out.push(')');
    }

    // @map
    if let Some(mapped_name) = field.database_name() {
        render_map_attribute(mapped_name, "@", out);
    }

    // @relation
    if let Field::RelationField(rf) = field {
        let mut args = Vec::new();
        let relation_info = &rf.relation_info;
        let parent_model = params.datamodel.find_model_by_relation_field_ref(rf).unwrap();
        let is_self_relation = relation_info.to.as_str() == model.name();

        if is_self_relation
            || relation_info.name != RelationNames::name_for_unambiguous_relation(&relation_info.to, &parent_model.name)
        {
            args.push((None, string_literal(&relation_info.name).to_string()));
        }

        if !relation_info.fields.is_empty() {
            args.push((Some("fields"), render_field_array(&relation_info.fields)));
        }

        if !relation_info.references.is_empty() {
            args.push((Some("references"), render_field_array(&relation_info.references)));
        }

        if let Some(ref_action) = relation_info.on_delete {
            if rf.default_on_delete_action() != ref_action {
                args.push((Some("onDelete"), ref_action.to_string()));
            }
        }

        if let Some(ref_action) = relation_info.on_update {
            if rf.default_on_update_action() != ref_action {
                args.push((Some("onUpdate"), ref_action.to_string()));
            }
        }

        if let Some(fk_name) = relation_info.fk_name.as_ref().filter(|s| !s.is_empty()) {
            if let Some(src) = params.datasource {
                if !foreign_key_name_matches(relation_info, parent_model, src.active_connector) {
                    args.push((Some("map"), string_literal(fk_name).to_string()));
                }
            };
        }

        if !args.is_empty() {
            out.push_str(" @relation");
            render_arguments(&mut args, out);
        }
    }

    if let (Some((scalar_type, native_type)), Some(datasource)) =
        (field.field_type().as_native_type(), params.datasource)
    {
        render_native_type_attribute(scalar_type, native_type, datasource, out);
    }

    // @ignore
    if field.is_ignored() {
        out.push_str(" @ignore");
    }
}

fn render_native_type_attribute(
    scalar_type: &ScalarType,
    native_type: &NativeTypeInstance,
    datasource: &Datasource,
    out: &mut String,
) {
    if datasource.active_connector.native_type_is_default_for_scalar_type(
        native_type.serialized_native_type.clone(),
        &dml_scalar_type_to_parser_database_scalar_type(*scalar_type),
    ) {
        return;
    }

    out.push_str(" @");
    out.push_str(&datasource.name);
    out.push('.');
    out.push_str(&native_type.name);

    let mut args = native_type.args.iter().map(|arg| (None, arg.clone())).collect();
    render_arguments(&mut args, out);
}

fn render_field_array(fields: &[String]) -> String {
    let mut out = String::from("[");
    let mut fields = fields.iter().peekable();
    while let Some(f) = fields.next() {
        out.push_str(f);
        if fields.peek().is_some() {
            out.push(',');
        }
    }
    out.push(']');
    out
}

fn render_pk_field_array(fields: &[PrimaryKeyField]) -> String {
    let mut out = String::from("[");
    let mut fields = fields.iter().peekable();
    while let Some(f) = fields.next() {
        out.push_str(&f.name);

        let mut args = Vec::new();
        if let Some(length) = &f.length {
            args.push((Some("length"), length.to_string()));
        }
        if let Some(SortOrder::Desc) = &f.sort_order {
            args.push((Some("sort"), "Desc".to_string()));
        }
        render_arguments(&mut args, &mut out);

        if fields.peek().is_some() {
            out.push(',');
        }
    }
    out.push(']');
    out
}

fn render_index_field_array(fields: &[IndexField], index: &IndexDefinition) -> String {
    let mut out = String::new();
    out.push('[');
    let mut fields = fields.iter().peekable();
    while let Some(f) = fields.next() {
        let mut name_path = f.path.iter().peekable();
        while let Some((ident, _)) = name_path.next() {
            out.push_str(ident);
            if name_path.peek().is_some() {
                out.push('.')
            }
        }

        let mut args = Vec::new();
        if let Some(length) = &f.length {
            args.push((Some("length"), length.to_string()));
        }

        match f.sort_order {
            Some(SortOrder::Asc) if index.is_fulltext() => {
                args.push((Some("sort"), "Asc".to_string()));
            }
            Some(SortOrder::Desc) => {
                args.push((Some("sort"), "Desc".to_string()));
            }
            _ => (),
        }

        if let Some(opclass) = &f.operator_class {
            if let Some(raw_class) = opclass.as_raw() {
                args.push((Some("ops"), format!("raw({})", string_literal(raw_class))));
            } else {
                args.push((Some("ops"), opclass.to_string()));
            }
        }

        render_arguments(&mut args, &mut out);

        if fields.peek().is_some() {
            out.push(',');
        }
    }
    out.push(']');
    out
}

fn render_composite_field_type(field_type: &CompositeTypeFieldType, out: &mut String) {
    match field_type {
        CompositeTypeFieldType::CompositeType(name) | CompositeTypeFieldType::Enum(name) => out.push_str(name),
        CompositeTypeFieldType::Unsupported(name) => {
            write!(out, "Unsupported({})", string_literal(name)).unwrap();
        }
        CompositeTypeFieldType::Scalar(tpe, _) => {
            out.push_str(&tpe.to_string());
        }
    }
}

fn render_field_type(field_type: &crate::FieldType, out: &mut String) {
    match field_type {
        FieldType::CompositeType(name) | FieldType::Enum(name) => out.push_str(name),
        FieldType::Unsupported(name) => {
            write!(out, "Unsupported({})", string_literal(name)).unwrap();
        }
        FieldType::Scalar(tpe, _) => {
            out.push_str(&tpe.to_string());
        }
        FieldType::Relation(rel) => out.push_str(&rel.to),
    }
}

fn render_model_attributes(model: &crate::Model, params: RenderParams<'_>, out: &mut String) {
    // @@id
    if let Some(pk) = &model.primary_key {
        if !pk.defined_on_field {
            out.push_str("@@id");
            let mut args = vec![(None, render_pk_field_array(&pk.fields))];

            if let Some(name) = &pk.name {
                args.push((Some("name"), string_literal(name).to_string()));
            }

            if let (Some(db_name), Some(src)) = (pk.db_name.as_ref().filter(|s| !s.is_empty()), params.datasource) {
                if !primary_key_name_matches(pk, model, src.active_connector) {
                    args.push((Some("map"), string_literal(db_name).to_string()));
                }
            }

            if matches!(pk.clustered, Some(false)) {
                args.push((Some("clustered"), "false".to_owned()));
            }

            render_arguments(&mut args, out);
            out.push('\n');
        }
    }

    let model_indexes = model
        .indices
        .iter()
        .filter(|idx| !(idx.is_unique() && idx.defined_on_field));
    for index_def in model_indexes {
        let attr = match index_def.tpe {
            IndexType::Normal => "@@index",
            IndexType::Unique => "@@unique",
            IndexType::Fulltext => "@@fulltext",
        };
        out.push_str(attr);
        let mut args = vec![(None, render_index_field_array(&index_def.fields, index_def))];
        push_index_arguments(index_def, model, params, &mut args);
        render_arguments(&mut args, out);
        out.push('\n');
    }

    // @@map
    if let Some(mapped_name) = &model.database_name {
        render_map_attribute(mapped_name, "@@", out);
        out.push('\n');
    }

    // @@ignore
    if model.is_ignored() {
        out.push_str("@@ignore\n");
    }
}

fn render_field_arity(arity: &crate::FieldArity, out: &mut String) {
    match arity {
        FieldArity::Required => (),
        FieldArity::Optional => out.push('?'),
        FieldArity::List => out.push_str("[]"),
    }
}

fn dml_scalar_type_to_parser_database_scalar_type(st: crate::ScalarType) -> db::ScalarType {
    match st {
        crate::ScalarType::Int => db::ScalarType::Int,
        crate::ScalarType::BigInt => db::ScalarType::BigInt,
        crate::ScalarType::Float => db::ScalarType::Float,
        crate::ScalarType::Boolean => db::ScalarType::Boolean,
        crate::ScalarType::String => db::ScalarType::String,
        crate::ScalarType::DateTime => db::ScalarType::DateTime,
        crate::ScalarType::Json => db::ScalarType::Json,
        crate::ScalarType::Bytes => db::ScalarType::Bytes,
        crate::ScalarType::Decimal => db::ScalarType::Decimal,
    }
}

fn render_default_value(dv: &crate::DefaultValue, out: &mut String) {
    match dv.kind() {
        crate::DefaultKind::Single(v) => render_prisma_value(v, out),
        crate::DefaultKind::Expression(e) => {
            out.push_str(e.name());
            out.push('(');
            let mut args = e.args().iter().peekable();
            while let Some((arg_name, value)) = args.next() {
                if let Some(name) = arg_name {
                    out.push_str(name);
                    out.push(':');
                }
                render_prisma_value(value, out);
                if args.peek().is_some() {
                    out.push_str(", ");
                }
            }
            out.push(')');
        }
    }
}

fn render_prisma_value(pv: &PrismaValue, out: &mut String) {
    match pv {
        PrismaValue::Boolean(true) => out.push_str("true"),
        PrismaValue::Boolean(false) => out.push_str("false"),
        PrismaValue::Uuid(value) => out.write_fmt(format_args!("{value}")).unwrap(),
        PrismaValue::DateTime(value) => out.write_fmt(format_args!("{value}")).unwrap(),
        PrismaValue::Xml(value) | PrismaValue::Json(value) | PrismaValue::String(value) => {
            out.write_fmt(format_args!("{}", string_literal(value))).unwrap();
        }
        PrismaValue::Enum(value) => out.push_str(value),
        PrismaValue::Float(value) => out.write_fmt(format_args!("{value}")).unwrap(),
        PrismaValue::BigInt(value) | PrismaValue::Int(value) => out.write_fmt(format_args!("{value}")).unwrap(),
        PrismaValue::List(vec) => {
            out.push('[');
            let mut values = vec.iter().peekable();
            while let Some(value) = values.next() {
                render_prisma_value(value, out);
                if values.peek().is_some() {
                    out.push_str(", ");
                }
            }
            out.push(']');
        }
        PrismaValue::Bytes(b) => write!(out, "{}", string_literal(&prisma_value::encode_bytes(b))).unwrap(),
        PrismaValue::Object(_) | PrismaValue::Null => unreachable!(),
    }
}

fn push_index_arguments(
    index: &IndexDefinition,
    model: &Model,
    params: RenderParams<'_>,
    args: &mut Vec<(Option<&'static str>, String)>,
) {
    if let Some(name) = &index.name {
        args.push((Some("name"), string_literal(name).to_string()));
    }

    if let Some(mapped_name) = index.db_name.as_ref().filter(|s| !s.is_empty()) {
        if !index_name_matches(index, params.datamodel, model, params.connector()) {
            args.push((Some("map"), string_literal(mapped_name).to_string()));
        }
    }

    match index.algorithm {
        Some(IndexAlgorithm::BTree) | None => (),
        Some(algo) => {
            args.push((Some("type"), algo.to_string()));
        }
    }

    if matches!(index.clustered, Some(true)) {
        args.push((Some("clustered"), "true".to_owned()));
    }
}

fn render_arguments(args: &mut Vec<(Option<&'static str>, String)>, out: &mut String) {
    if args.is_empty() {
        return;
    }

    out.push('(');
    let mut args = args.iter_mut().peekable();
    while let Some((name, value)) = args.next() {
        if let Some(name) = name {
            out.push_str(name);
            out.push_str(": ");
        }

        out.push_str(value);

        if args.peek().is_some() {
            out.push_str(", ");
        }
    }
    out.push(')');
}

fn foreign_key_name_matches(ri: &RelationInfo, model: &Model, connector: &dyn Connector) -> bool {
    let column_names: Vec<&str> = ri
        .fields
        .iter()
        .map(|field_name| {
            // We cannot unwrap here, due to us re-introspecting relations
            // and if we're not using foreign keys, we might copy a relation
            // that is not valid anymore. We still want to write that to the
            // file and let user fix it, but if we unwrap here, we will
            // panic.
            model
                .find_scalar_field(field_name)
                .map(|field| field.final_database_name())
                .unwrap_or(field_name)
        })
        .collect();

    ri.fk_name.as_ref().unwrap()
        == &ConstraintNames::foreign_key_constraint_name(model.final_database_name(), &column_names, connector)
}

fn primary_key_name_matches(pk: &PrimaryKeyDefinition, model: &Model, connector: &dyn Connector) -> bool {
    pk.db_name.as_ref().unwrap() == &ConstraintNames::primary_key_name(model.final_database_name(), connector)
}

fn index_name_matches(idx: &IndexDefinition, datamodel: &Datamodel, model: &Model, connector: &dyn Connector) -> bool {
    let column_names: Vec<Vec<(&str, Option<&str>)>> = idx
        .fields
        .iter()
        .map(|field| {
            field
                .path
                .iter()
                .map(|field_def| match field_def {
                    (field_name, Some(type_name)) => {
                        let field: &str = datamodel
                            .find_composite_type(type_name)
                            .and_then(|ct| ct.find_field(field_name))
                            .and_then(|field| field.database_name.as_deref())
                            .unwrap_or(field_name.as_str());

                        (field, Some(type_name.as_str()))
                    }
                    (field_name, None) => (
                        model
                            .find_scalar_field(field_name)
                            .map(|field| field.final_database_name())
                            .unwrap_or(field_name),
                        None,
                    ),
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let expected = if idx.is_unique() {
        ConstraintNames::unique_index_name(model.final_database_name(), &column_names, connector)
    } else {
        ConstraintNames::non_unique_index_name(model.final_database_name(), &column_names, connector)
    };

    idx.db_name.as_deref().unwrap() == expected
}
