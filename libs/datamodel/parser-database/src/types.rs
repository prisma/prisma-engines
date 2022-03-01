use crate::{context::Context, interner::StringId, DatamodelError};
use schema_ast::ast::{self, WithName};
use std::collections::{BTreeMap, HashMap};

pub(super) fn resolve_types(ctx: &mut Context<'_>) {
    for (top_id, top) in ctx.ast.iter_tops() {
        match (top_id, top) {
            (ast::TopId::Alias(alias_id), ast::Top::Type(type_alias)) => visit_type_alias(alias_id, type_alias, ctx),
            (ast::TopId::Model(model_id), ast::Top::Model(model)) => visit_model(model_id, model, ctx),
            (ast::TopId::Enum(_), ast::Top::Enum(enm)) => visit_enum(enm, ctx),
            (ast::TopId::CompositeType(ct_id), ast::Top::CompositeType(ct)) => visit_composite_type(ct_id, ct, ctx),
            (_, ast::Top::Source(_)) | (_, ast::Top::Generator(_)) => (),
            _ => unreachable!(),
        }
    }

    detect_alias_cycles(ctx);
}

#[derive(Debug, Default)]
pub(super) struct Types {
    pub(super) composite_type_fields: BTreeMap<(ast::CompositeTypeId, ast::FieldId), CompositeTypeField>,
    pub(super) type_aliases: HashMap<ast::AliasId, ScalarFieldType>,
    pub(super) scalar_fields: BTreeMap<(ast::ModelId, ast::FieldId), ScalarField>,
    /// This contains only the relation fields actually present in the schema
    /// source text.
    pub(super) relation_fields: BTreeMap<(ast::ModelId, ast::FieldId), RelationField>,
    pub(super) enum_attributes: HashMap<ast::EnumId, EnumAttributes>,
    pub(super) model_attributes: HashMap<ast::ModelId, ModelAttributes>,
}

impl Types {
    pub(super) fn range_model_scalar_fields(
        &self,
        model_id: ast::ModelId,
    ) -> impl Iterator<Item = (ast::FieldId, &ScalarField)> {
        self.scalar_fields
            .range((model_id, ast::FieldId::MIN)..=(model_id, ast::FieldId::MAX))
            .map(|((_, field_id), scalar_field)| (*field_id, scalar_field))
    }

    pub(super) fn take_scalar_field(&mut self, model_id: ast::ModelId, field_id: ast::FieldId) -> Option<ScalarField> {
        self.scalar_fields.remove(&(model_id, field_id))
    }

    pub(super) fn take_relation_field(
        &mut self,
        model_id: ast::ModelId,
        field_id: ast::FieldId,
    ) -> Option<RelationField> {
        self.relation_fields.remove(&(model_id, field_id))
    }
}

#[derive(Debug, Clone)]
pub(super) struct CompositeTypeField {
    pub(super) r#type: ScalarFieldType,
    pub(super) mapped_name: Option<StringId>,
    pub(super) default: Option<DefaultAttribute>,
    /// Native type name and arguments
    ///
    /// (attribute scope, native type name, arguments, span)
    ///
    /// For example: `@db.Text` would translate to ("db", "Text", &[], <the span>)
    pub(crate) native_type: Option<(StringId, StringId, Vec<String>, ast::Span)>,
}

#[derive(Debug)]
enum FieldType {
    Model(ast::ModelId),
    Scalar(ScalarFieldType),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnsupportedType {
    name: StringId,
}

impl UnsupportedType {
    pub(crate) fn new(name: StringId) -> Self {
        Self { name }
    }
}

/// The type of a scalar field, parsed and categorized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScalarFieldType {
    /// A composite type
    CompositeType(ast::CompositeTypeId),
    /// An enum
    Enum(ast::EnumId),
    /// A Prisma scalar type
    BuiltInScalar(ScalarType),
    /// A type alias
    Alias(ast::AliasId),
    /// An `Unsupported("...")` type
    Unsupported(UnsupportedType),
}

impl ScalarFieldType {
    /// Try to interpret this field type as a known Prisma scalar type.
    pub fn as_builtin_scalar(self) -> Option<ScalarType> {
        match self {
            ScalarFieldType::BuiltInScalar(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DefaultAttribute {
    pub(crate) mapped_name: Option<StringId>,
    pub(crate) argument_idx: usize,
    pub(crate) default_attribute: ast::AttributeId,
}

#[derive(Debug)]
pub(crate) struct ScalarField {
    pub(crate) r#type: ScalarFieldType,
    pub(crate) is_ignored: bool,
    pub(crate) is_updated_at: bool,
    pub(crate) default: Option<DefaultAttribute>,
    /// @map
    pub(crate) mapped_name: Option<StringId>,
    /// Native type name and arguments
    ///
    /// (attribute scope, native type name, arguments, span)
    ///
    /// For example: `@db.Text` would translate to ("db", "Text", &[], <the span>)
    pub(crate) native_type: Option<(StringId, StringId, Vec<String>, ast::Span)>,
}

#[derive(Debug)]
pub(crate) struct RelationField {
    pub(crate) referenced_model: ast::ModelId,
    pub(crate) on_delete: Option<(crate::ReferentialAction, ast::Span)>,
    pub(crate) on_update: Option<(crate::ReferentialAction, ast::Span)>,
    /// The fields _explicitly present_ in the AST.
    pub(crate) fields: Option<Vec<ast::FieldId>>,
    /// The `references` fields _explicitly present_ in the AST.
    pub(crate) references: Option<Vec<ast::FieldId>>,
    /// The name _explicitly present_ in the AST.
    pub(crate) name: Option<StringId>,
    pub(crate) is_ignored: bool,
    /// The foreign key name _explicitly present_ in the AST through the `@map` attribute.
    pub(crate) mapped_name: Option<StringId>,
    pub(crate) relation_attribute: Option<ast::AttributeId>,
}

impl RelationField {
    fn new(referenced_model: ast::ModelId) -> Self {
        RelationField {
            referenced_model,
            on_delete: None,
            on_update: None,
            fields: None,
            references: None,
            name: None,
            is_ignored: false,
            mapped_name: None,
            relation_attribute: None,
        }
    }
}

/// Information gathered from validating attributes on a model.
#[derive(Default, Debug)]
pub(crate) struct ModelAttributes {
    /// @(@)id
    pub(super) primary_key: Option<IdAttribute>,
    /// @@ignore
    pub(crate) is_ignored: bool,
    /// @@index and @(@)unique explicitely written to the schema AST.
    pub(super) ast_indexes: Vec<(ast::AttributeId, IndexAttribute)>,
    /// @(@)unique added implicitely to the datamodel by us.
    pub(super) implicit_indexes: Vec<IndexAttribute>,
    /// @@map
    pub(crate) mapped_name: Option<StringId>,
}

/// A type of index as defined by the `type: ...` argument on an index attribute.
///
/// ```ignore
/// @@index([a, b], type: Hash)
///                 ^^^^^^^^^^
/// ```
#[derive(Debug, Clone, Copy)]
pub enum IndexAlgorithm {
    /// Binary tree index (the default in most databases)
    BTree,
    /// Hash index
    Hash,
}

impl IndexAlgorithm {
    /// Is this a hash index?
    pub fn is_hash(self) -> bool {
        matches!(self, Self::Hash)
    }
}

impl Default for IndexAlgorithm {
    fn default() -> Self {
        Self::BTree
    }
}

/// The different types of indexes supported in the Prisma Schema Language.
#[derive(Debug, Clone, Copy)]
pub enum IndexType {
    /// @@index
    Normal,
    /// @(@)unique
    Unique,
    /// @(@)fulltext
    Fulltext,
}

impl Default for IndexType {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Default)]
pub(crate) struct IndexAttribute {
    pub(crate) r#type: IndexType,
    pub(crate) fields: Vec<FieldWithArgs>,
    pub(crate) source_field: Option<ast::FieldId>,
    pub(crate) name: Option<StringId>,
    pub(crate) mapped_name: Option<StringId>,
    pub(crate) algorithm: Option<IndexAlgorithm>,
}

impl IndexAttribute {
    pub(crate) fn is_unique(&self) -> bool {
        matches!(self.r#type, IndexType::Unique)
    }

    pub(crate) fn is_fulltext(&self) -> bool {
        matches!(self.r#type, IndexType::Fulltext)
    }

    pub(crate) fn fields_match(&self, other: &[ast::FieldId]) -> bool {
        self.fields.len() == other.len() && self.fields.iter().zip(other.iter()).all(|(a, b)| a.field_id == *b)
    }
}

#[derive(Debug)]
pub(crate) struct IdAttribute {
    pub(crate) fields: Vec<FieldWithArgs>,
    pub(super) source_field: Option<ast::FieldId>,
    pub(super) source_attribute: ast::AttributeId,
    pub(super) name: Option<StringId>,
    pub(super) mapped_name: Option<StringId>,
}

impl IdAttribute {
    pub(crate) fn fields_match(&self, other: &[ast::FieldId]) -> bool {
        self.fields.len() == other.len() && self.fields.iter().zip(other.iter()).all(|(a, b)| a.field_id == *b)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FieldWithArgs {
    pub(crate) field_id: ast::FieldId,
    pub(crate) sort_order: Option<SortOrder>,
    pub(crate) length: Option<u32>,
}

#[derive(Debug, Default)]
pub(super) struct EnumAttributes {
    pub(super) mapped_name: Option<StringId>,
    /// @map on enum values.
    pub(super) mapped_values: HashMap<u32, StringId>,
}

fn visit_model<'db>(model_id: ast::ModelId, ast_model: &'db ast::Model, ctx: &mut Context<'db>) {
    for (field_id, ast_field) in ast_model.iter_fields() {
        match field_type(ast_field, ctx) {
            Ok(FieldType::Model(referenced_model)) => {
                let rf = RelationField::new(referenced_model);
                ctx.types.relation_fields.insert((model_id, field_id), rf);
            }
            Ok(FieldType::Scalar(scalar_field_type)) => {
                let field_data = ScalarField {
                    r#type: scalar_field_type,
                    is_ignored: false,
                    is_updated_at: false,
                    default: None,
                    mapped_name: None,
                    native_type: None,
                };

                ctx.types.scalar_fields.insert((model_id, field_id), field_data);
            }
            Err(supported) => ctx.push_error(DatamodelError::new_type_not_found_error(
                supported,
                ast_field.field_type.span(),
            )),
        }
    }
}

/// Detect self-referencing type aliases, possibly indirectly. We loop
/// through each type alias in the schema. If it references another type
/// alias — which may in turn reference another type alias —, we check that
/// it is not self-referencing. If a type alias ends up transitively
/// referencing itself, we create an error diagnostic.
fn detect_alias_cycles(ctx: &mut Context<'_>) {
    // The IDs of the type aliases we traversed to get to the current type alias.
    let mut path = Vec::new();
    // We accumulate the errors here because we want to sort them at the end.
    let mut errors: Vec<(ast::AliasId, DatamodelError)> = Vec::new();

    for (alias_id, ty) in &ctx.types.type_aliases {
        // Loop variable. This is the "tip" of the sequence of type aliases.
        let mut current = (*alias_id, ty);
        path.clear();

        // Follow the chain of type aliases referencing other type aliases.
        while let ScalarFieldType::Alias(next_alias_id) = current.1 {
            path.push(current.0);
            let next_alias = &ctx.ast[*next_alias_id];
            // Detect a cycle where next type is also the root. In that
            // case, we want to report an error.
            if path.len() > 1 && &path[0] == next_alias_id {
                errors.push((
                    *alias_id,
                    DatamodelError::new_validation_error(
                        format!(
                            "Recursive type definitions are not allowed. Recursive path was: {} -> {}.",
                            path.iter()
                                .map(|id| ctx.ast[*id].name.name.as_str())
                                .collect::<Vec<_>>()
                                .join(" -> "),
                            &next_alias.name.name,
                        ),
                        next_alias.field_type.span(),
                    ),
                ));
                break;
            }

            // We detect a cycle anywhere else in the chain of type aliases.
            // In that case, the error will be reported somewhere else, and
            // we can just move on from this alias.
            if path.contains(next_alias_id) {
                break;
            }

            match ctx.types.type_aliases.get(next_alias_id) {
                Some(next_alias_type) => {
                    current = (*next_alias_id, next_alias_type);
                }
                // A missing alias at this point means that there was an
                // error resolving the type of the next alias. We stop
                // validation here.
                None => break,
            }
        }
    }

    errors.sort_by_key(|(id, _err)| *id);
    for (_, error) in errors {
        ctx.push_error(error);
    }
}

fn visit_composite_type<'db>(ct_id: ast::CompositeTypeId, ct: &'db ast::CompositeType, ctx: &mut Context<'db>) {
    for (field_id, ast_field) in ct.iter_fields() {
        match field_type(ast_field, ctx) {
            Ok(FieldType::Scalar(ScalarFieldType::Alias(_))) => {
                ctx.push_error(DatamodelError::new_composite_type_validation_error(
                    "Type aliases are not allowed on composite types. Consider using the resolved type instead."
                        .to_string(),
                    ct.name.name.clone(),
                    ast_field.field_type.span(),
                ))
            }
            Ok(FieldType::Scalar(scalar_type)) => {
                let field = CompositeTypeField {
                    r#type: scalar_type,
                    mapped_name: None,
                    default: None,
                    native_type: None,
                };
                ctx.types.composite_type_fields.insert((ct_id, field_id), field);
            }
            Ok(FieldType::Model(referenced_model_id)) => {
                let referenced_model_name = ctx.ast[referenced_model_id].name();
                ctx.push_error(DatamodelError::new_composite_type_validation_error(format!("{} refers to a model, making this a relation field. Relation fields inside composite types are not supported.", referenced_model_name), ct.name.name.clone(), ast_field.field_type.span()))
            }
            Err(supported) => ctx.push_error(DatamodelError::new_type_not_found_error(
                supported,
                ast_field.field_type.span(),
            )),
        }
    }
}

fn visit_enum<'db>(enm: &'db ast::Enum, ctx: &mut Context<'db>) {
    if enm.values.is_empty() {
        ctx.push_error(DatamodelError::new_validation_error(
            "An enum must have at least one value.".to_owned(),
            enm.span,
        ))
    }
}

fn visit_type_alias<'db>(alias_id: ast::AliasId, alias: &'db ast::Field, ctx: &mut Context<'db>) {
    match field_type(alias, ctx) {
        Ok(FieldType::Scalar(scalar_field_type)) => {
            ctx.types.type_aliases.insert(alias_id, scalar_field_type);
        }
        Ok(FieldType::Model(_)) => ctx.push_error(DatamodelError::new_validation_error(
            "Only scalar types can be used for defining custom types.".to_owned(),
            alias.field_type.span(),
        )),
        Err(supported) => ctx.push_error(DatamodelError::new_type_not_found_error(
            supported,
            alias.field_type.span(),
        )),
    };
}

/// Either a structured, supported type, or an Err(unsupported) if the type name
/// does not match any we know of.
fn field_type<'db>(field: &'db ast::Field, ctx: &mut Context<'db>) -> Result<FieldType, &'db str> {
    let supported = match &field.field_type {
        ast::FieldType::Supported(ident) => &ident.name,
        ast::FieldType::Unsupported(name, _) => {
            let unsupported = UnsupportedType::new(ctx.interner.intern(name));
            return Ok(FieldType::Scalar(ScalarFieldType::Unsupported(unsupported)));
        }
    };
    let supported_string_id = ctx.interner.intern(supported);

    if let Some(tpe) = ScalarType::try_from_str(supported) {
        return Ok(FieldType::Scalar(ScalarFieldType::BuiltInScalar(tpe)));
    }

    match ctx.names.tops.get(&supported_string_id).map(|id| (*id, &ctx.ast[*id])) {
        Some((ast::TopId::Model(model_id), ast::Top::Model(_))) => Ok(FieldType::Model(model_id)),
        Some((ast::TopId::Enum(enum_id), ast::Top::Enum(_))) => Ok(FieldType::Scalar(ScalarFieldType::Enum(enum_id))),
        Some((ast::TopId::Alias(id), ast::Top::Type(_))) => Ok(FieldType::Scalar(ScalarFieldType::Alias(id))),
        Some((ast::TopId::CompositeType(ctid), ast::Top::CompositeType(_))) => {
            Ok(FieldType::Scalar(ScalarFieldType::CompositeType(ctid)))
        }
        Some((_, ast::Top::Generator(_))) | Some((_, ast::Top::Source(_))) => unreachable!(),
        None => Err(supported),
        _ => unreachable!(),
    }
}

/// The sort order of an index.
#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    /// ASCending
    Asc,
    /// DESCending
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Asc
    }
}

/// Prisma's builtin scalar types.
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
#[allow(missing_docs)]
pub enum ScalarType {
    Int,
    BigInt,
    Float,
    Boolean,
    String,
    DateTime,
    Json,
    Bytes,
    Decimal,
}

impl ScalarType {
    /// The string representation of the scalar type in the schema.
    pub fn as_str(&self) -> &'static str {
        match self {
            ScalarType::Int => "Int",
            ScalarType::BigInt => "BigInt",
            ScalarType::Float => "Float",
            ScalarType::Boolean => "Boolean",
            ScalarType::String => "String",
            ScalarType::DateTime => "DateTime",
            ScalarType::Json => "Json",
            ScalarType::Bytes => "Bytes",
            ScalarType::Decimal => "Decimal",
        }
    }

    pub(crate) fn try_from_str(s: &str) -> Option<ScalarType> {
        match s {
            "Int" => Some(ScalarType::Int),
            "BigInt" => Some(ScalarType::BigInt),
            "Float" => Some(ScalarType::Float),
            "Boolean" => Some(ScalarType::Boolean),
            "String" => Some(ScalarType::String),
            "DateTime" => Some(ScalarType::DateTime),
            "Json" => Some(ScalarType::Json),
            "Bytes" => Some(ScalarType::Bytes),
            "Decimal" => Some(ScalarType::Decimal),
            _ => None,
        }
    }
}
