use crate::{context::Context, walkers::CompositeTypeFieldWalker, walkers::CompositeTypeWalker, DatamodelError};
use schema_ast::ast::{self, WithName};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    rc::Rc,
};

pub(super) fn resolve_types(ctx: &mut Context<'_>) {
    for (top_id, top) in ctx.db.ast.iter_tops() {
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
    detect_composite_cycles(ctx);
}

#[derive(Debug, Default)]
pub(super) struct Types<'ast> {
    pub(super) composite_type_fields: BTreeMap<(ast::CompositeTypeId, ast::FieldId), CompositeTypeField<'ast>>,
    pub(super) type_aliases: HashMap<ast::AliasId, ScalarFieldType>,
    pub(super) scalar_fields: BTreeMap<(ast::ModelId, ast::FieldId), ScalarField<'ast>>,
    /// This contains only the relation fields actually present in the schema
    /// source text.
    pub(super) relation_fields: BTreeMap<(ast::ModelId, ast::FieldId), RelationField<'ast>>,
    pub(super) enum_attributes: HashMap<ast::EnumId, EnumAttributes<'ast>>,
    pub(super) model_attributes: HashMap<ast::ModelId, ModelAttributes<'ast>>,
}

impl<'ast> Types<'ast> {
    pub(super) fn range_model_scalar_fields(
        &self,
        model_id: ast::ModelId,
    ) -> impl Iterator<Item = (ast::FieldId, &ScalarField<'ast>)> {
        self.scalar_fields
            .range((model_id, ast::FieldId::ZERO)..=(model_id, ast::FieldId::MAX))
            .map(|((_, field_id), scalar_field)| (*field_id, scalar_field))
    }

    pub(super) fn take_scalar_field(
        &mut self,
        model_id: ast::ModelId,
        field_id: ast::FieldId,
    ) -> Option<ScalarField<'ast>> {
        self.scalar_fields.remove(&(model_id, field_id))
    }

    pub(super) fn take_relation_field(
        &mut self,
        model_id: ast::ModelId,
        field_id: ast::FieldId,
    ) -> Option<RelationField<'ast>> {
        self.relation_fields.remove(&(model_id, field_id))
    }
}

#[derive(Debug)]
pub(super) struct CompositeTypeField<'ast> {
    pub(super) r#type: ScalarFieldType,
    pub(super) mapped_name: Option<&'ast str>,
    pub(super) default: Option<DefaultAttribute<'ast>>,
}

#[derive(Debug)]
enum FieldType {
    Model(ast::ModelId),
    Scalar(ScalarFieldType),
}

/// The type of a scalar field, parsed and categorized.
#[derive(Debug, Clone, Copy)]
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
    Unsupported,
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

#[derive(Debug)]
pub(crate) struct DefaultAttribute<'ast> {
    pub(crate) mapped_name: Option<&'ast str>,
    pub(crate) value: &'ast ast::Expression,
    pub(crate) default_attribute: &'ast ast::Attribute,
}

#[derive(Debug)]
pub(crate) struct ScalarField<'ast> {
    pub(crate) r#type: ScalarFieldType,
    pub(crate) is_ignored: bool,
    pub(crate) is_updated_at: bool,
    pub(crate) default: Option<DefaultAttribute<'ast>>,
    /// @map
    pub(crate) mapped_name: Option<&'ast str>,
    /// Native type name and arguments
    ///
    /// (attribute scope, native type name, arguments, span)
    ///
    /// For example: `@db.Text` would translate to ("db", "Text", &[], <the span>)
    pub(crate) native_type: Option<(&'ast str, &'ast str, Vec<String>, ast::Span)>,
}

#[derive(Debug)]
pub(crate) struct RelationField<'ast> {
    pub(crate) referenced_model: ast::ModelId,
    pub(crate) on_delete: Option<(crate::ReferentialAction, ast::Span)>,
    pub(crate) on_update: Option<(crate::ReferentialAction, ast::Span)>,
    /// The fields _explicitly present_ in the AST.
    pub(crate) fields: Option<Vec<ast::FieldId>>,
    /// The `references` fields _explicitly present_ in the AST.
    pub(crate) references: Option<Vec<ast::FieldId>>,
    /// The name _explicitly present_ in the AST.
    pub(crate) name: Option<&'ast str>,
    pub(crate) is_ignored: bool,
    /// The foreign key name _explicitly present_ in the AST through the `@map` attribute.
    pub(crate) mapped_name: Option<&'ast str>,
    pub(crate) relation_attribute: Option<&'ast ast::Attribute>,
}

impl RelationField<'_> {
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
pub(crate) struct ModelAttributes<'ast> {
    /// @(@)id
    pub(super) primary_key: Option<IdAttribute<'ast>>,
    /// @@ignore
    pub(crate) is_ignored: bool,
    /// @@index and @(@)unique explicitely written to the schema AST.
    pub(super) ast_indexes: Vec<(&'ast ast::Attribute, IndexAttribute<'ast>)>,
    /// @(@)unique added implicitely to the datamodel by us.
    pub(super) implicit_indexes: Vec<IndexAttribute<'static>>,
    /// @@map
    pub(crate) mapped_name: Option<&'ast str>,
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
pub(crate) struct IndexAttribute<'ast> {
    pub(crate) r#type: IndexType,
    pub(crate) fields: Vec<FieldWithArgs>,
    pub(crate) source_field: Option<ast::FieldId>,
    pub(crate) name: Option<&'ast str>,
    pub(crate) mapped_name: Option<&'ast str>,
    pub(crate) algorithm: Option<IndexAlgorithm>,
}

impl<'ast> IndexAttribute<'ast> {
    pub(crate) fn is_unique(&self) -> bool {
        matches!(self.r#type, IndexType::Unique)
    }

    pub(crate) fn is_fulltext(&self) -> bool {
        matches!(self.r#type, IndexType::Fulltext)
    }
}

#[derive(Debug)]
pub(crate) struct IdAttribute<'ast> {
    pub(crate) fields: Vec<FieldWithArgs>,
    pub(super) source_field: Option<ast::FieldId>,
    pub(super) source_attribute: &'ast ast::Attribute,
    pub(super) name: Option<&'ast str>,
    pub(super) mapped_name: Option<&'ast str>,
}

#[derive(Debug)]
pub struct FieldWithArgs {
    pub(crate) field_id: ast::FieldId,
    pub(crate) sort_order: Option<SortOrder>,
    pub(crate) length: Option<u32>,
}

#[derive(Debug, Default)]
pub(super) struct EnumAttributes<'ast> {
    pub(super) mapped_name: Option<&'ast str>,
    /// @map on enum values.
    pub(super) mapped_values: HashMap<u32, &'ast str>,
}

fn visit_model<'ast>(model_id: ast::ModelId, ast_model: &'ast ast::Model, ctx: &mut Context<'ast>) {
    for (field_id, ast_field) in ast_model.iter_fields() {
        match field_type(ast_field, ctx) {
            Ok(FieldType::Model(referenced_model)) => {
                let rf = RelationField::new(referenced_model);
                ctx.db.types.relation_fields.insert((model_id, field_id), rf);
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

                ctx.db.types.scalar_fields.insert((model_id, field_id), field_data);
            }
            Err(supported) => ctx.push_error(DatamodelError::new_type_not_found_error(
                supported,
                ast_field.field_type.span(),
            )),
        }
    }
}

struct CompositeTypePath<'ast, 'db> {
    previous: Option<Rc<CompositeTypePath<'ast, 'db>>>,
    current: CompositeTypeWalker<'ast, 'db>,
}

impl<'ast, 'db> CompositeTypePath<'ast, 'db> {
    fn root(current: CompositeTypeWalker<'ast, 'db>) -> Self {
        Self {
            previous: None,
            current,
        }
    }

    fn link(self: &Rc<Self>, current: CompositeTypeWalker<'ast, 'db>) -> Self {
        Self {
            previous: Some(self.clone()),
            current,
        }
    }
}

impl<'ast, 'db> fmt::Display for CompositeTypePath<'ast, 'db> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut traversed = vec![self.current];
        let mut this = self;

        while let Some(next) = this.previous.as_ref() {
            traversed.push(next.current);
            this = next;
        }

        let path = traversed
            .into_iter()
            .map(|w| w.name())
            .map(|n| format!("`{}`", n))
            .collect::<Vec<_>>()
            .join(" → ");

        f.write_str(&path)
    }
}

/// Detect compound type chains that form a cycle, that is not broken with either an optional or an
/// array type.
fn detect_composite_cycles(ctx: &mut Context<'_>) {
    let mut visited: Vec<ast::CompositeTypeId> = Vec::new();
    let mut errors: Vec<(ast::CompositeTypeId, DatamodelError)> = Vec::new();

    let mut fields_to_traverse: Vec<(CompositeTypeFieldWalker<'_, '_>, Option<Rc<CompositeTypePath<'_, '_>>>)> = ctx
        .db
        .walk_composite_types()
        .flat_map(|ct| ct.fields())
        .filter(|f| f.arity().is_required())
        .map(|f| (f, None))
        .collect();

    while let Some((field, path)) = fields_to_traverse.pop() {
        let path = match path {
            Some(path) => path,
            None => {
                visited.clear();
                Rc::new(CompositeTypePath::root(field.composite_type()))
            }
        };

        match field.r#type() {
            ScalarFieldType::CompositeType(ctid) if field.composite_type().composite_type_id() == *ctid => {
                let msg = "The type is the same as the parent and causes an endless cycle. Please change the field to be either optional or a list.";
                errors.push((
                    *ctid,
                    DatamodelError::new_composite_type_field_validation_error(
                        msg,
                        field.composite_type().name(),
                        field.name(),
                        field.ast_field().span,
                    ),
                ));
            }
            ScalarFieldType::CompositeType(ctid) if visited.first() == Some(ctid) => {
                let msg = format!(
                    "The types cause an endless cycle in the path {}. Please change one of the fields to be either optional or a list to break the cycle.",
                    path,
                );

                errors.push((
                    *ctid,
                    DatamodelError::new_composite_type_field_validation_error(
                        &msg,
                        field.composite_type().name(),
                        field.name(),
                        field.ast_field().span,
                    ),
                ));
            }
            ScalarFieldType::CompositeType(ctid) => {
                visited.push(*ctid);

                for field in ctx
                    .db
                    .walk_composite_type(*ctid)
                    .fields()
                    .filter(|f| f.arity().is_required())
                {
                    let path = Rc::new(path.link(field.composite_type()));
                    fields_to_traverse.push((field, Some(path)));
                }
            }
            _ => (),
        }
    }

    errors.sort_by_key(|(id, _err)| *id);
    for (_, error) in errors {
        ctx.push_error(error);
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

    for (alias_id, ty) in &ctx.db.types.type_aliases {
        // Loop variable. This is the "tip" of the sequence of type aliases.
        let mut current = (*alias_id, ty);
        path.clear();

        // Follow the chain of type aliases referencing other type aliases.
        while let ScalarFieldType::Alias(next_alias_id) = current.1 {
            path.push(current.0);
            let next_alias = &ctx.db.ast[*next_alias_id];
            // Detect a cycle where next type is also the root. In that
            // case, we want to report an error.
            if path.len() > 1 && &path[0] == next_alias_id {
                errors.push((
                    *alias_id,
                    DatamodelError::new_validation_error(
                        format!(
                            "Recursive type definitions are not allowed. Recursive path was: {} -> {}.",
                            path.iter()
                                .map(|id| ctx.db.ast[*id].name.name.as_str())
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

            match ctx.db.types.type_aliases.get(next_alias_id) {
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

fn visit_composite_type<'ast>(ct_id: ast::CompositeTypeId, ct: &'ast ast::CompositeType, ctx: &mut Context<'ast>) {
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
                };
                ctx.db.types.composite_type_fields.insert((ct_id, field_id), field);
            }
            Ok(FieldType::Model(referenced_model_id)) => {
                let referenced_model_name = ctx.db.ast[referenced_model_id].name();
                ctx.push_error(DatamodelError::new_composite_type_validation_error(format!("{} refers to a model, making this a relation field. Relation fields inside composite types are not supported.", referenced_model_name), ct.name.name.clone(), ast_field.field_type.span()))
            }
            Err(supported) => ctx.push_error(DatamodelError::new_type_not_found_error(
                supported,
                ast_field.field_type.span(),
            )),
        }
    }
}

fn visit_enum<'ast>(enm: &'ast ast::Enum, ctx: &mut Context<'ast>) {
    if enm.values.is_empty() {
        ctx.push_error(DatamodelError::new_validation_error(
            "An enum must have at least one value.".to_owned(),
            enm.span,
        ))
    }
}

fn visit_type_alias<'ast>(alias_id: ast::AliasId, alias: &'ast ast::Field, ctx: &mut Context<'ast>) {
    match field_type(alias, ctx) {
        Ok(FieldType::Scalar(scalar_field_type)) => {
            ctx.db.types.type_aliases.insert(alias_id, scalar_field_type);
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
fn field_type<'ast>(field: &'ast ast::Field, ctx: &mut Context<'ast>) -> Result<FieldType, &'ast str> {
    let supported = match &field.field_type {
        ast::FieldType::Supported(ident) => &ident.name,
        ast::FieldType::Unsupported(_, _) => return Ok(FieldType::Scalar(ScalarFieldType::Unsupported)),
    };

    if let Some(tpe) = ScalarType::try_from_str(supported) {
        return Ok(FieldType::Scalar(ScalarFieldType::BuiltInScalar(tpe)));
    }

    match ctx
        .db
        .names
        .tops
        .get(supported.as_str())
        .map(|id| (*id, &ctx.db.ast[*id]))
    {
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
