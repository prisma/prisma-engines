pub(crate) mod index_fields;

use crate::{context::Context, interner::StringId, walkers::IndexFieldWalker, DatamodelError};
use either::Either;
use enumflags2::bitflags;
use schema_ast::ast::{self, WithName};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

pub(super) fn resolve_types(ctx: &mut Context<'_>) {
    for (top_id, top) in ctx.ast.iter_tops() {
        match (top_id, top) {
            (ast::TopId::Model(model_id), ast::Top::Model(model)) => visit_model(model_id, model, ctx),
            (ast::TopId::Enum(_), ast::Top::Enum(enm)) => visit_enum(enm, ctx),
            (ast::TopId::CompositeType(ct_id), ast::Top::CompositeType(ct)) => visit_composite_type(ct_id, ct, ctx),
            (_, ast::Top::Source(_)) | (_, ast::Top::Generator(_)) => (),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Types {
    pub(super) composite_type_fields: BTreeMap<(ast::CompositeTypeId, ast::FieldId), CompositeTypeField>,
    scalar_fields: Vec<ScalarField>,
    /// This contains only the relation fields actually present in the schema
    /// source text.
    relation_fields: Vec<RelationField>,
    pub(super) enum_attributes: HashMap<ast::EnumId, EnumAttributes>,
    pub(super) model_attributes: HashMap<ast::ModelId, ModelAttributes>,
    /// Sorted array of scalar fields that have an `@default()` attribute with a function that is
    /// not part of the base Prisma ones. This is meant for later validation in the datamodel
    /// connector.
    pub(super) unknown_function_defaults: Vec<ScalarFieldId>,
}

impl Types {
    pub(super) fn find_model_scalar_field(
        &self,
        model_id: ast::ModelId,
        field_id: ast::FieldId,
    ) -> Option<ScalarFieldId> {
        self.scalar_fields
            .binary_search_by_key(&(model_id, field_id), |sf| (sf.model_id, sf.field_id))
            .ok()
            .map(|idx| ScalarFieldId(idx as u32))
    }

    pub(super) fn range_model_scalar_fields(
        &self,
        model_id: ast::ModelId,
    ) -> impl Iterator<Item = (ScalarFieldId, &ScalarField)> {
        let start = self.scalar_fields.partition_point(|sf| sf.model_id < model_id);
        self.scalar_fields[start..]
            .iter()
            .take_while(move |sf| sf.model_id == model_id)
            .enumerate()
            .map(move |(idx, sf)| (ScalarFieldId((start + idx) as u32), sf))
    }

    pub(super) fn iter_relation_field_ids(&self) -> impl Iterator<Item = RelationFieldId> + 'static {
        (0..self.relation_fields.len()).map(|idx| RelationFieldId(idx as u32))
    }

    pub(super) fn iter_relation_fields(&self) -> impl Iterator<Item = (RelationFieldId, &RelationField)> {
        self.relation_fields
            .iter()
            .enumerate()
            .map(|(idx, rf)| (RelationFieldId(idx as u32), rf))
    }

    pub(super) fn range_model_scalar_field_ids(
        &self,
        model_id: ast::ModelId,
    ) -> impl Iterator<Item = ScalarFieldId> + Clone {
        let end = self.scalar_fields.partition_point(|sf| sf.model_id <= model_id);
        let start = self.scalar_fields[..end].partition_point(|sf| sf.model_id < model_id);
        (start..end).map(|idx| ScalarFieldId(idx as u32))
    }

    pub(super) fn range_model_relation_fields(
        &self,
        model_id: ast::ModelId,
    ) -> impl Iterator<Item = (RelationFieldId, &RelationField)> + Clone {
        let first_relation_field_idx = self.relation_fields.partition_point(|rf| rf.model_id < model_id);
        self.relation_fields[first_relation_field_idx..]
            .iter()
            .take_while(move |rf| rf.model_id == model_id)
            .enumerate()
            .map(move |(idx, rf)| (RelationFieldId((first_relation_field_idx + idx) as u32), rf))
    }

    pub(super) fn refine_field(&self, id: (ast::ModelId, ast::FieldId)) -> Either<RelationFieldId, ScalarFieldId> {
        self.relation_fields
            .binary_search_by_key(&id, |rf| (rf.model_id, rf.field_id))
            .map(|idx| Either::Left(RelationFieldId(idx as u32)))
            .or_else(|_| {
                self.scalar_fields
                    .binary_search_by_key(&id, |sf| (sf.model_id, sf.field_id))
                    .map(|id| Either::Right(ScalarFieldId(id as u32)))
            })
            .expect("expected field to be either scalar or relation field")
    }

    pub(super) fn push_relation_field(&mut self, relation_field: RelationField) -> RelationFieldId {
        let id = RelationFieldId(self.relation_fields.len() as u32);
        self.relation_fields.push(relation_field);
        id
    }

    pub(super) fn push_scalar_field(&mut self, scalar_field: ScalarField) -> ScalarFieldId {
        let id = ScalarFieldId(self.scalar_fields.len() as u32);
        self.scalar_fields.push(scalar_field);
        id
    }
}

impl std::ops::Index<RelationFieldId> for Types {
    type Output = RelationField;

    fn index(&self, index: RelationFieldId) -> &Self::Output {
        &self.relation_fields[index.0 as usize]
    }
}

impl std::ops::IndexMut<RelationFieldId> for Types {
    fn index_mut(&mut self, index: RelationFieldId) -> &mut Self::Output {
        &mut self.relation_fields[index.0 as usize]
    }
}

impl std::ops::Index<ScalarFieldId> for Types {
    type Output = ScalarField;

    fn index(&self, index: ScalarFieldId) -> &Self::Output {
        &self.scalar_fields[index.0 as usize]
    }
}

impl std::ops::IndexMut<ScalarFieldId> for Types {
    fn index_mut(&mut self, index: ScalarFieldId) -> &mut Self::Output {
        &mut self.scalar_fields[index.0 as usize]
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

    /// Try to interpret this field type as a Composite Type.
    pub fn as_composite_type(self) -> Option<ast::CompositeTypeId> {
        match self {
            ScalarFieldType::CompositeType(id) => Some(id),
            _ => None,
        }
    }

    /// Try to interpret this field type as an enum.
    pub fn as_enum(self) -> Option<ast::EnumId> {
        match self {
            ScalarFieldType::Enum(id) => Some(id),
            _ => None,
        }
    }

    /// Is the type of the field `Unsupported("...")`?
    pub fn is_unsupported(self) -> bool {
        matches!(self, Self::Unsupported(_))
    }

    /// True if the field's type is Json.
    pub fn is_json(self) -> bool {
        matches!(self, Self::BuiltInScalar(ScalarType::Json))
    }

    /// True if the field's type is String.
    pub fn is_string(self) -> bool {
        matches!(self, Self::BuiltInScalar(ScalarType::String))
    }

    /// True if the field's type is Bytes.
    pub fn is_bytes(self) -> bool {
        matches!(self, Self::BuiltInScalar(ScalarType::Bytes))
    }

    /// True if the field's type is DateTime.
    pub fn is_datetime(self) -> bool {
        matches!(self, Self::BuiltInScalar(ScalarType::DateTime))
    }

    /// True if the field's type is Float.
    pub fn is_float(self) -> bool {
        matches!(self, Self::BuiltInScalar(ScalarType::Float))
    }

    /// True if the field's type is Int.
    pub fn is_int(self) -> bool {
        matches!(self, Self::BuiltInScalar(ScalarType::Int))
    }

    /// True if the field's type is BigInt.
    pub fn is_bigint(self) -> bool {
        matches!(self, Self::BuiltInScalar(ScalarType::BigInt))
    }

    /// True if the field's type is Decimal.
    pub fn is_decimal(self) -> bool {
        matches!(self, Self::BuiltInScalar(ScalarType::Decimal))
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
    pub(crate) model_id: ast::ModelId,
    pub(crate) field_id: ast::FieldId,
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
    pub(crate) model_id: ast::ModelId,
    pub(crate) field_id: ast::FieldId,
    pub(crate) referenced_model: ast::ModelId,
    pub(crate) on_delete: Option<(crate::ReferentialAction, ast::Span)>,
    pub(crate) on_update: Option<(crate::ReferentialAction, ast::Span)>,
    /// The fields _explicitly present_ in the AST.
    pub(crate) fields: Option<Vec<ScalarFieldId>>,
    /// The `references` fields _explicitly present_ in the AST.
    pub(crate) references: Option<Vec<ScalarFieldId>>,
    /// The name _explicitly present_ in the AST.
    pub(crate) name: Option<StringId>,
    pub(crate) is_ignored: bool,
    /// The foreign key name _explicitly present_ in the AST through the `@map` attribute.
    pub(crate) mapped_name: Option<StringId>,
    pub(crate) relation_attribute: Option<ast::AttributeId>,
}

impl RelationField {
    fn new(model_id: ast::ModelId, field_id: ast::FieldId, referenced_model: ast::ModelId) -> Self {
        RelationField {
            model_id,
            field_id,
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
    /// @@map
    pub(crate) mapped_name: Option<StringId>,
    /// ```ignore
    /// @@schema("public")
    ///          ^^^^^^^^
    /// ```
    pub(crate) schema: Option<(StringId, ast::Span)>,
}

/// A type of index as defined by the `type: ...` argument on an index attribute.
///
/// ```ignore
/// @@index([a, b], type: Hash)
///                 ^^^^^^^^^^
/// ```
#[bitflags]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexAlgorithm {
    /// Binary tree index (the default in most databases)
    BTree,
    /// Hash index
    Hash,
    /// GiST index
    Gist,
    /// GIN index
    Gin,
    /// SP-GiST index
    SpGist,
    /// Brin index
    Brin,
}

impl fmt::Display for IndexAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexAlgorithm::BTree => f.write_str("BTree"),
            IndexAlgorithm::Hash => f.write_str("Hash"),
            IndexAlgorithm::Gist => f.write_str("Gist"),
            IndexAlgorithm::Gin => f.write_str("Gin"),
            IndexAlgorithm::SpGist => f.write_str("SpGist"),
            IndexAlgorithm::Brin => f.write_str("Brin"),
        }
    }
}

impl IndexAlgorithm {
    /// Is this a B-Tree index?
    pub fn is_btree(self) -> bool {
        matches!(self, Self::BTree)
    }

    /// Hash?
    pub fn is_hash(self) -> bool {
        matches!(self, Self::Hash)
    }

    /// GiST?
    pub fn is_gist(self) -> bool {
        matches!(self, Self::Gist)
    }

    /// GIN?
    pub fn is_gin(self) -> bool {
        matches!(self, Self::Gin)
    }

    /// SP-GiST?
    pub fn is_spgist(self) -> bool {
        matches!(self, Self::SpGist)
    }

    /// BRIN?
    pub fn is_brin(self) -> bool {
        matches!(self, Self::Brin)
    }

    /// True if the operator class can be used with the given scalar type.
    pub fn supports_field_type(self, field: IndexFieldWalker<'_>) -> bool {
        let r#type = field.scalar_field_type();

        if r#type.is_unsupported() {
            return true;
        }

        match self {
            IndexAlgorithm::BTree => true,
            IndexAlgorithm::Hash => true,
            IndexAlgorithm::Gist => r#type.is_string(),
            IndexAlgorithm::Gin => r#type.is_json() || field.ast_field().arity.is_list(),
            IndexAlgorithm::SpGist => r#type.is_string(),
            IndexAlgorithm::Brin => {
                r#type.is_string()
                    || r#type.is_bytes()
                    || r#type.is_datetime()
                    || r#type.is_float()
                    || r#type.is_int()
                    || r#type.is_bigint()
                    || r#type.is_decimal()
            }
        }
    }

    /// Documentation for editor autocompletion.
    pub fn documentation(self) -> &'static str {
        match self {
            IndexAlgorithm::BTree => "Can handle equality and range queries on data that can be sorted into some ordering (default).",
            IndexAlgorithm::Hash => "Can handle simple equality queries, but no ordering. Faster than BTree, if ordering is not needed.",
            IndexAlgorithm::Gist => "Generalized Search Tree. A framework for building specialized indices for custom data types.",
            IndexAlgorithm::Gin => "Generalized Inverted Index. Useful for indexing composite items, such as arrays or text.",
            IndexAlgorithm::SpGist => "Space-partitioned Generalized Search Tree. For implenting a wide range of different non-balanced data structures.",
            IndexAlgorithm::Brin => "Block Range Index. If the data has some natural correlation with their physical location within the table, can compress very large amount of data into a small space.",
        }
    }
}

impl Default for IndexAlgorithm {
    fn default() -> Self {
        Self::BTree
    }
}

/// The different types of indexes supported in the Prisma Schema Language.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum IndexType {
    /// @@index
    #[default]
    Normal,
    /// @(@)unique
    Unique,
    /// @(@)fulltext
    Fulltext,
}

#[derive(Debug, Default)]
pub(crate) struct IndexAttribute {
    pub(crate) r#type: IndexType,
    pub(crate) fields: Vec<FieldWithArgs>,
    pub(crate) source_field: Option<ScalarFieldId>,
    pub(crate) name: Option<StringId>,
    pub(crate) mapped_name: Option<StringId>,
    pub(crate) algorithm: Option<IndexAlgorithm>,
    pub(crate) clustered: Option<bool>,
}

impl IndexAttribute {
    pub(crate) fn is_unique(&self) -> bool {
        matches!(self.r#type, IndexType::Unique)
    }

    pub(crate) fn is_fulltext(&self) -> bool {
        matches!(self.r#type, IndexType::Fulltext)
    }

    pub(crate) fn is_normal(&self) -> bool {
        matches!(self.r#type, IndexType::Normal)
    }
}

#[derive(Debug)]
pub(crate) struct IdAttribute {
    pub(crate) fields: Vec<FieldWithArgs>,
    pub(super) source_field: Option<ast::FieldId>,
    pub(super) source_attribute: ast::AttributeId,
    pub(super) name: Option<StringId>,
    pub(super) mapped_name: Option<StringId>,
    pub(super) clustered: Option<bool>,
}

/// Defines a path to a field that is not directly in the model.
///
/// ```ignore
/// type A {
///   field String
/// }
///
/// model A {
///   id Int @id
///   a  A
///
///   @@index([a.field])
///   //       ^this thing here, path separated with `.`
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct IndexFieldPath {
    /// The field in the model that starts the path to the final field included
    /// in the index. The type of this field has to be a composite type.
    ///
    /// ```ignore
    /// type A {
    ///   field String
    /// }
    ///
    /// model A {
    ///   id Int @id
    ///   a  A
    /// //^this one is the root
    ///   @@index([a.field])
    /// }
    /// ```
    root: ScalarFieldId,
    /// Path from the root, one composite type at a time. The final item is the
    /// field that gets included in the index.
    ///
    /// ```ignore
    /// type A {
    ///   field String
    /// }
    ///
    /// model A {
    ///   id Int @id
    ///   a  A
    ///   @@index([a.field])
    /// //           ^this one is the path. in this case a vector of one element
    /// }
    /// ```
    path: Vec<(ast::CompositeTypeId, ast::FieldId)>,
}

impl IndexFieldPath {
    pub(crate) fn new(root: ScalarFieldId) -> Self {
        Self { root, path: Vec::new() }
    }

    pub(crate) fn push_field(&mut self, ctid: ast::CompositeTypeId, field_id: ast::FieldId) {
        self.path.push((ctid, field_id));
    }

    /// The starting point of the index path. If the indexed field is not in a
    /// composite type, returns the same value as [`field_in_index`](Self::field_in_index()).
    ///
    /// ```ignore
    /// type A {
    ///   field String
    /// }
    ///
    /// model A {
    ///   id Int @id
    ///   a  A
    /// //^here
    ///
    ///   @@index([a.field])
    /// }
    /// ```
    pub fn root(&self) -> ScalarFieldId {
        self.root
    }

    /// The path after [`root`](Self::root()). Empty if the field is not pointing to a
    /// composite type.
    ///
    /// ```ignore
    /// type A {
    ///   field String
    /// //^the path is a vector of one element, pointing to this field.
    /// }
    ///
    /// model A {
    ///   id Int @id
    ///   a  A
    ///
    ///   @@index([a.field])
    /// }
    /// ```
    pub fn path(&self) -> &[(ast::CompositeTypeId, ast::FieldId)] {
        &self.path
    }

    /// The field that gets included in the index. Can either be in the model,
    /// or in a composite type embedded in the model. Returns the same value as
    /// the [`root`](Self::root()) method if the field is in a model rather than in a
    /// composite type.
    pub fn field_in_index(&self) -> Either<ScalarFieldId, (ast::CompositeTypeId, ast::FieldId)> {
        self.path
            .last()
            .map(|id| Either::Right(*id))
            .unwrap_or(Either::Left(self.root))
    }
}

#[derive(Debug, Clone)]
pub struct FieldWithArgs {
    pub(crate) path: IndexFieldPath,
    pub(crate) sort_order: Option<SortOrder>,
    pub(crate) length: Option<u32>,
    pub(crate) operator_class: Option<OperatorClassStore>,
}

#[derive(Debug, Default)]
pub(super) struct EnumAttributes {
    pub(super) mapped_name: Option<StringId>,
    /// @map on enum values.
    pub(super) mapped_values: HashMap<u32, StringId>,
    /// ```ignore
    /// @@schema("public")
    ///          ^^^^^^^^
    /// ```
    pub(crate) schema: Option<(StringId, ast::Span)>,
}

fn visit_model<'db>(model_id: ast::ModelId, ast_model: &'db ast::Model, ctx: &mut Context<'db>) {
    for (field_id, ast_field) in ast_model.iter_fields() {
        match field_type(ast_field, ctx) {
            Ok(FieldType::Model(referenced_model)) => {
                let rf = RelationField::new(model_id, field_id, referenced_model);
                ctx.types.push_relation_field(rf);
            }
            Ok(FieldType::Scalar(scalar_field_type)) => {
                ctx.types.push_scalar_field(ScalarField {
                    model_id,
                    field_id,
                    r#type: scalar_field_type,
                    is_ignored: false,
                    is_updated_at: false,
                    default: None,
                    mapped_name: None,
                    native_type: None,
                });
            }
            Err(supported) => ctx.push_error(DatamodelError::new_type_not_found_error(
                supported,
                ast_field.field_type.span(),
            )),
        }
    }
}

fn visit_composite_type<'db>(ct_id: ast::CompositeTypeId, ct: &'db ast::CompositeType, ctx: &mut Context<'db>) {
    for (field_id, ast_field) in ct.iter_fields() {
        match field_type(ast_field, ctx) {
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
                ctx.push_error(DatamodelError::new_composite_type_validation_error(&format!("{referenced_model_name} refers to a model, making this a relation field. Relation fields inside composite types are not supported."), ct.name(), ast_field.field_type.span()))
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
        let msg = "An enum must have at least one value.";
        ctx.push_error(DatamodelError::new_validation_error(msg, enm.span))
    }
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
        Some((ast::TopId::CompositeType(ctid), ast::Top::CompositeType(_))) => {
            Ok(FieldType::Scalar(ScalarFieldType::CompositeType(ctid)))
        }
        Some((_, ast::Top::Generator(_))) | Some((_, ast::Top::Source(_))) => unreachable!(),
        None => Err(supported),
        _ => unreachable!(),
    }
}

/// Defines operators captured by the index. Used with PostgreSQL
/// GiST/SP-GiST/GIN/BRIN indices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperatorClass {
    /// An operator class for `Gist` index and `inet` type.
    ///
    /// # Indexable Operators
    ///
    /// - `<< (inet, inet)`
    /// - `<<= (inet, inet)`
    /// - `>> (inet, inet)`
    /// - `>>= (inet, inet)`
    /// - `= (inet, inet)`
    /// - `<> (inet, inet)`
    /// - `< (inet, inet)`
    /// - `<= (inet, inet)`
    /// - `> (inet, inet)`
    /// - `>= (inet, inet)`
    /// - `&& (inet, inet)`
    InetOps,
    /// An operator class for `Gin` index and `jsonb` type.
    ///
    /// # Indexable Operators
    ///
    /// - `@> (jsonb,jsonb)`
    /// - `@? (jsonb,jsonpath)`
    /// - `@@ (jsonb,jsonpath)`
    /// - `? (jsonb,text)`
    /// - `?| (jsonb,text[])`
    /// - `?& (jsonb,text[])`
    JsonbOps,
    /// An operator class for `Gin` index and `jsonb` type.
    ///
    /// # Indexable Operators
    ///
    /// - `@> (jsonb,jsonb)`
    /// - `@? (jsonb,jsonpath)`
    /// - `@@ (jsonb,jsonpath)`
    JsonbPathOps,
    /// An operator class for `Gin` index and any array type.
    ///
    /// # Indexable Operators
    ///
    /// - `&& (anyarray,anyarray)`
    /// - `@> (anyarray,anyarray)`
    /// - `<@ (anyarray,anyarray)`
    /// - `= (anyarray,anyarray)`
    ArrayOps,
    /// An operator class for `SpGist` index and `text`, `char` and
    /// `varchar` types.
    ///
    /// # Indexable Operators
    ///
    /// - `= (text,text)`
    /// - `< (text,text)`
    /// - `<= (text,text)`
    /// - `> (text,text)`
    /// - `>= (text,text)`
    /// - `~<~ (text,text)`
    /// - `~<=~ (text,text)`
    /// - `~>=~ (text,text)`
    /// - `~>~ (text,text)`
    /// - `^@ (text,text)`
    TextOps,
    /// An operator class for `Brin` index and `bit` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (bit,bit)`
    /// - `< (bit,bit)`
    /// - `> (bit,bit)`
    /// - `<= (bit,bit)`
    /// - `>= (bit,bit)`
    BitMinMaxOps,
    /// An operator class for `Brin` index and `varbit` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (varbit,varbit)`
    /// - `< (varbit,varbit)`
    /// - `> (varbit,varbit)`
    /// - `<= (varbit,varbit)`
    /// - `>= (varbit,varbit)`
    VarBitMinMaxOps,
    /// An operator class for `Brin` index and `char` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (character,character)`
    BpcharBloomOps,
    /// An operator class for `Brin` index and `char` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (character,character)`
    /// - `< (character,character)`
    /// - `<= (character,character)`
    /// - `> (character,character)`
    /// - `>= (character,character)`
    BpcharMinMaxOps,
    /// An operator class for `Brin` index and `bytea` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (bytea,bytea)`
    ByteaBloomOps,
    /// An operator class for `Brin` index and `bytea` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (bytea,bytea)`
    /// - `< (bytea,bytea)`
    /// - `<= (bytea,bytea)`
    /// - `> (bytea,bytea)`
    /// - `>= (bytea,bytea)`
    ByteaMinMaxOps,
    /// An operator class for `Brin` index and `date` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (date,date)`
    DateBloomOps,
    /// An operator class for `Brin` index and `date` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (date,date)`
    /// - `< (date,date)`
    /// - `<= (date,date)`
    /// - `> (date,date)`
    /// - `>= (date,date)`
    DateMinMaxOps,
    /// An operator class for `Brin` index and `date` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (date,date)`
    /// - `< (date,date)`
    /// - `<= (date,date)`
    /// - `> (date,date)`
    /// - `>= (date,date)`
    DateMinMaxMultiOps,
    /// An operator class for `Brin` index and `real` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (float4,float4)`
    Float4BloomOps,
    /// An operator class for `Brin` index and `real` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (float4,float4)`
    /// - `< (float4,float4)`
    /// - `> (float4,float4)`
    /// - `<= (float4,float4)`
    /// - `>= (float4,float4)`
    Float4MinMaxOps,
    /// An operator class for `Brin` index and `real` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (float4,float4)`
    /// - `< (float4,float4)`
    /// - `> (float4,float4)`
    /// - `<= (float4,float4)`
    /// - `>= (float4,float4)`
    Float4MinMaxMultiOps,
    /// An operator class for `Brin` index and `double precision` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (float8,float8)`
    Float8BloomOps,
    /// An operator class for `Brin` index and `double precision` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (float8,float8)`
    /// - `< (float8,float8)`
    /// - `> (float8,float8)`
    /// - `<= (float8,float8)`
    /// - `>= (float8,float8)`
    Float8MinMaxOps,
    /// An operator class for `Brin` index and `double precision` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (float8,float8)`
    /// - `< (float8,float8)`
    /// - `> (float8,float8)`
    /// - `<= (float8,float8)`
    /// - `>= (float8,float8)`
    Float8MinMaxMultiOps,
    /// An operator class for `Brin` index and `inet` type.
    ///
    /// # Indexable Operators
    ///
    /// - `<< (inet,inet)`
    /// - `<<= (inet,inet)`
    /// - `>> (inet,inet)`
    /// - `>>= (inet,inet)`
    /// - `= (inet,inet)`
    /// - `&& (inet,inet)`
    InetInclusionOps,
    /// An operator class for `Brin` index and `inet` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (inet,inet)`
    InetBloomOps,
    /// An operator class for `Brin` index and `inet` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (inet,inet)`
    /// - `< (inet,inet)`
    /// - `> (inet,inet)`
    /// - `<= (inet,inet)`
    /// - `>= (inet,inet)`
    InetMinMaxOps,
    /// An operator class for `Brin` index and `inet` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (inet,inet)`
    /// - `< (inet,inet)`
    /// - `> (inet,inet)`
    /// - `<= (inet,inet)`
    /// - `>= (inet,inet)`
    InetMinMaxMultiOps,
    /// An operator class for `Brin` index and `int2` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (int2,int2)`
    Int2BloomOps,
    /// An operator class for `Brin` index and `int2` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (int2,int2)`
    /// - `< (int2,int2)`
    /// - `> (int2,int2)`
    /// - `<= (int2,int2)`
    /// - `>= (int2,int2)`
    Int2MinMaxOps,
    /// An operator class for `Brin` index and `int2` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (int2,int2)`
    /// - `< (int2,int2)`
    /// - `> (int2,int2)`
    /// - `<= (int2,int2)`
    /// - `>= (int2,int2)`
    Int2MinMaxMultiOps,
    /// An operator class for `Brin` index and `int4` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (int4,int4)`
    Int4BloomOps,
    /// An operator class for `Brin` index and `int4` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (int4,int4)`
    /// - `< (int4,int4)`
    /// - `> (int4,int4)`
    /// - `<= (int4,int4)`
    /// - `>= (int4,int4)`
    Int4MinMaxOps,
    /// An operator class for `Brin` index and `int4` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (int4,int4)`
    /// - `< (int4,int4)`
    /// - `> (int4,int4)`
    /// - `<= (int4,int4)`
    /// - `>= (int4,int4)`
    Int4MinMaxMultiOps,
    /// An operator class for `Brin` index and `int8` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (int8,int8)`
    Int8BloomOps,
    /// An operator class for `Brin` index and `int8` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (int8,int8)`
    /// - `< (int8,int8)`
    /// - `> (int8,int8)`
    /// - `<= (int8,int8)`
    /// - `>= (int8,int8)`
    Int8MinMaxOps,
    /// An operator class for `Brin` index and `int8` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (int8,int8)`
    /// - `< (int8,int8)`
    /// - `> (int8,int8)`
    /// - `<= (int8,int8)`
    /// - `>= (int8,int8)`
    Int8MinMaxMultiOps,
    /// An operator class for `Brin` index and `numeric` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (numeric,numeric)`
    NumericBloomOps,
    /// An operator class for `Brin` index and `numeric` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (numeric,numeric)`
    /// - `< (numeric,numeric)`
    /// - `> (numeric,numeric)`
    /// - `<= (numeric,numeric)`
    /// - `>= (numeric,numeric)`
    NumericMinMaxOps,
    /// An operator class for `Brin` index and `numeric` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (numeric,numeric)`
    /// - `< (numeric,numeric)`
    /// - `> (numeric,numeric)`
    /// - `<= (numeric,numeric)`
    /// - `>= (numeric,numeric)`
    NumericMinMaxMultiOps,
    /// An operator class for `Brin` index and `oid` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (oid,oid)`
    OidBloomOps,
    /// An operator class for `Brin` index and `oid` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (oid,oid)`
    /// - `< (oid,oid)`
    /// - `> (oid,oid)`
    /// - `<= (oid,oid)`
    /// - `>= (oid,oid)`
    OidMinMaxOps,
    /// An operator class for `Brin` index and `oid` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (oid,oid)`
    /// - `< (oid,oid)`
    /// - `> (oid,oid)`
    /// - `<= (oid,oid)`
    /// - `>= (oid,oid)`
    OidMinMaxMultiOps,
    /// An operator class for `Brin` index and `text` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (text,text)`
    TextBloomOps,
    /// An operator class for `Brin` index and `text` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (text,text)`
    /// - `< (text,text)`
    /// - `> (text,text)`
    /// - `<= (text,text)`
    /// - `>= (text,text)`
    TextMinMaxOps,
    /// An operator class for `Brin` index and `timestamp` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (timestamp,timestamp)`
    TimestampBloomOps,
    /// An operator class for `Brin` index and `timestamp` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (timestamp,timestamp)`
    /// - `< (timestamp,timestamp)`
    /// - `> (timestamp,timestamp)`
    /// - `<= (timestamp,timestamp)`
    /// - `>= (timestamp,timestamp)`
    TimestampMinMaxOps,
    /// An operator class for `Brin` index and `timestamp` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (timestamp,timestamp)`
    /// - `< (timestamp,timestamp)`
    /// - `> (timestamp,timestamp)`
    /// - `<= (timestamp,timestamp)`
    /// - `>= (timestamp,timestamp)`
    TimestampMinMaxMultiOps,
    /// An operator class for `Brin` index and `timestamptz` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (timestamptz,timestamptz)`
    TimestampTzBloomOps,
    /// An operator class for `Brin` index and `timestamptz` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (timestamptz,timestamptz)`
    /// - `< (timestamptz,timestamptz)`
    /// - `> (timestamptz,timestamptz)`
    /// - `<= (timestamptz,timestamptz)`
    /// - `>= (timestamptz,timestamptz)`
    TimestampTzMinMaxOps,
    /// An operator class for `Brin` index and `timestamptz` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (timestamptz,timestamptz)`
    /// - `< (timestamptz,timestamptz)`
    /// - `> (timestamptz,timestamptz)`
    /// - `<= (timestamptz,timestamptz)`
    /// - `>= (timestamptz,timestamptz)`
    TimestampTzMinMaxMultiOps,
    /// An operator class for `Brin` index and `time` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (time,time)`
    TimeBloomOps,
    /// An operator class for `Brin` index and `time` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (time,time)`
    /// - `< (time,time)`
    /// - `> (time,time)`
    /// - `<= (time,time)`
    /// - `>= (time,time)`
    TimeMinMaxOps,
    /// An operator class for `Brin` index and `time` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (time,time)`
    /// - `< (time,time)`
    /// - `> (time,time)`
    /// - `<= (time,time)`
    /// - `>= (time,time)`
    TimeMinMaxMultiOps,
    /// An operator class for `Brin` index and `timetz` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (timetz,timetz)`
    TimeTzBloomOps,
    /// An operator class for `Brin` index and `timetz` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (timetz,timetz)`
    /// - `< (timetz,timetz)`
    /// - `> (timetz,timetz)`
    /// - `<= (timetz,timetz)`
    /// - `>= (timetz,timetz)`
    TimeTzMinMaxOps,
    /// An operator class for `Brin` index and `timetz` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (timetz,timetz)`
    /// - `< (timetz,timetz)`
    /// - `> (timetz,timetz)`
    /// - `<= (timetz,timetz)`
    /// - `>= (timetz,timetz)`
    TimeTzMinMaxMultiOps,
    /// An operator class for `Brin` index and `uuid` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (uuid,uuid)`
    UuidBloomOps,
    /// An operator class for `Brin` index and `uuid` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (uuid,uuid)`
    /// - `< (uuid,uuid)`
    /// - `> (uuid,uuid)`
    /// - `<= (uuid,uuid)`
    /// - `>= (uuid,uuid)`
    UuidMinMaxOps,
    /// An operator class for `Brin` index and `uuid` type.
    ///
    /// # Indexable Operators
    ///
    /// - `= (uuid,uuid)`
    /// - `< (uuid,uuid)`
    /// - `> (uuid,uuid)`
    /// - `<= (uuid,uuid)`
    /// - `>= (uuid,uuid)`
    UuidMinMaxMultiOps,
}

impl OperatorClass {
    /// True if the class supports the given index type.
    pub fn supports_index_type(self, algo: IndexAlgorithm) -> bool {
        match self {
            Self::InetOps => matches!(algo, IndexAlgorithm::Gist | IndexAlgorithm::SpGist),
            Self::JsonbOps => matches!(algo, IndexAlgorithm::Gin),
            Self::JsonbPathOps => matches!(algo, IndexAlgorithm::Gin),
            Self::ArrayOps => matches!(algo, IndexAlgorithm::Gin),
            Self::TextOps => matches!(algo, IndexAlgorithm::SpGist),
            Self::BitMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::VarBitMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::BpcharBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::BpcharMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::ByteaBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::ByteaMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::DateBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::DateMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::DateMinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Float4BloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Float4MinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Float4MinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Float8BloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Float8MinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Float8MinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::InetInclusionOps => matches!(algo, IndexAlgorithm::Brin),
            Self::InetBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::InetMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::InetMinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Int2BloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Int2MinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Int2MinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Int4BloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Int4MinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Int4MinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Int8BloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Int8MinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::Int8MinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::NumericBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::NumericMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::NumericMinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::OidBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::OidMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::OidMinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TextBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TextMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimestampBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimestampMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimestampMinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimestampTzBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimestampTzMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimestampTzMinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimeBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimeMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimeMinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimeTzBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimeTzMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::TimeTzMinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
            Self::UuidBloomOps => matches!(algo, IndexAlgorithm::Brin),
            Self::UuidMinMaxOps => matches!(algo, IndexAlgorithm::Brin),
            Self::UuidMinMaxMultiOps => matches!(algo, IndexAlgorithm::Brin),
        }
    }
}

impl fmt::Display for OperatorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InetOps => f.write_str("InetOps"),
            Self::JsonbOps => f.write_str("JsonbOps"),
            Self::JsonbPathOps => f.write_str("JsonbPathOps"),
            Self::ArrayOps => f.write_str("ArrayOps"),
            Self::TextOps => f.write_str("TextOps"),
            Self::BitMinMaxOps => f.write_str("BitMinMaxOps"),
            Self::VarBitMinMaxOps => f.write_str("VarBitMinMaxOps"),
            Self::BpcharBloomOps => f.write_str("BpcharBloomOps"),
            Self::BpcharMinMaxOps => f.write_str("BpcharMinMaxOps"),
            Self::ByteaBloomOps => f.write_str("ByteaBloomOps"),
            Self::ByteaMinMaxOps => f.write_str("ByteaMinMaxOps"),
            Self::DateBloomOps => f.write_str("DateBloomOps"),
            Self::DateMinMaxOps => f.write_str("DateMinMaxOps"),
            Self::DateMinMaxMultiOps => f.write_str("DateMinMaxMultiOps"),
            Self::Float4BloomOps => f.write_str("Float4BloomOps"),
            Self::Float4MinMaxOps => f.write_str("Float4MinMaxOps"),
            Self::Float4MinMaxMultiOps => f.write_str("Float4MinMaxMultiOps"),
            Self::Float8BloomOps => f.write_str("Float8BloomOps"),
            Self::Float8MinMaxOps => f.write_str("Float8MinMaxOps"),
            Self::Float8MinMaxMultiOps => f.write_str("Float8MinMaxMultiOps"),
            Self::InetInclusionOps => f.write_str("InetInclusionOps"),
            Self::InetBloomOps => f.write_str("InetBloomOps"),
            Self::InetMinMaxOps => f.write_str("InetMinMaxOps"),
            Self::InetMinMaxMultiOps => f.write_str("InetMinMaxMultiOps"),
            Self::Int2BloomOps => f.write_str("Int2BloomOps"),
            Self::Int2MinMaxOps => f.write_str("Int2MinMaxOps"),
            Self::Int2MinMaxMultiOps => f.write_str("Int2MinMaxMultiOps"),
            Self::Int4BloomOps => f.write_str("Int4BloomOps"),
            Self::Int4MinMaxOps => f.write_str("Int4MinMaxOps"),
            Self::Int4MinMaxMultiOps => f.write_str("Int4MinMaxMultiOps"),
            Self::Int8BloomOps => f.write_str("Int8BloomOps"),
            Self::Int8MinMaxOps => f.write_str("Int8MinMaxOps"),
            Self::Int8MinMaxMultiOps => f.write_str("Int8MinMaxMultiOps"),
            Self::NumericBloomOps => f.write_str("NumericBloomOps"),
            Self::NumericMinMaxOps => f.write_str("NumericMinMaxOps"),
            Self::NumericMinMaxMultiOps => f.write_str("NumericMinMaxMultiOps"),
            Self::OidBloomOps => f.write_str("OidBloomOps"),
            Self::OidMinMaxOps => f.write_str("OidMinMaxOps"),
            Self::OidMinMaxMultiOps => f.write_str("OidMinMaxMultiOps"),
            Self::TextBloomOps => f.write_str("TextBloomOps"),
            Self::TextMinMaxOps => f.write_str("TextMinMaxOps"),
            Self::TimestampBloomOps => f.write_str("TimestampBloomOps"),
            Self::TimestampMinMaxOps => f.write_str("TimestampMinMaxOps"),
            Self::TimestampMinMaxMultiOps => f.write_str("TimestampMinMaxMultiOps"),
            Self::TimestampTzBloomOps => f.write_str("TimestampTzBloomOps"),
            Self::TimestampTzMinMaxOps => f.write_str("TimestampTzMinMaxOps"),
            Self::TimestampTzMinMaxMultiOps => f.write_str("TimestampTzMinMaxMultiOps"),
            Self::TimeBloomOps => f.write_str("TimeBloomOps"),
            Self::TimeMinMaxOps => f.write_str("TimeMinMaxOps"),
            Self::TimeMinMaxMultiOps => f.write_str("TimeMinMaxMultiOps"),
            Self::TimeTzBloomOps => f.write_str("TimeTzBloomOps"),
            Self::TimeTzMinMaxOps => f.write_str("TimeTzMinMaxOps"),
            Self::TimeTzMinMaxMultiOps => f.write_str("TimeTzMinMaxMultiOps"),
            Self::UuidBloomOps => f.write_str("UuidBloomOps"),
            Self::UuidMinMaxOps => f.write_str("UuidMinMaxOps"),
            Self::UuidMinMaxMultiOps => f.write_str("BitMinMaxOps"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OperatorClassStore {
    pub(crate) inner: Either<OperatorClass, StringId>,
}

impl From<OperatorClass> for OperatorClassStore {
    fn from(inner: OperatorClass) -> Self {
        Self {
            inner: Either::Left(inner),
        }
    }
}

impl OperatorClassStore {
    pub(crate) fn raw(id: StringId) -> Self {
        Self {
            inner: Either::Right(id),
        }
    }
}

/// The sort order of an index.
#[derive(Debug, Clone, Copy, PartialEq)]
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

    /// True if the type is bytes.
    pub fn is_bytes(&self) -> bool {
        matches!(self, ScalarType::Bytes)
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

/// An opaque identifier for a model relation field in a schema.
#[derive(Copy, Clone, PartialEq, Debug, Hash, Eq, PartialOrd, Ord)]
pub struct RelationFieldId(u32);

/// An opaque identifier for a model scalar field in a schema.
#[derive(Copy, Clone, PartialEq, Debug, Eq, Hash)]
pub struct ScalarFieldId(u32);
