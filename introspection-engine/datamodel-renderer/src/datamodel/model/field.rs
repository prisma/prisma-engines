use crate::{
    datamodel::{
        attributes::FieldAttribute, model::index_field_input::IndexFieldOptions, DefaultValue, FieldType,
        IdFieldDefinition, Relation,
    },
    value::{Constant, Documentation, Function, Text},
};
use psl::dml;
use std::{borrow::Cow, collections::HashMap, fmt};

/// A field in a model block.
#[derive(Debug)]
pub struct ModelField<'a> {
    name: Constant<Cow<'a, str>>,
    commented_out: bool,
    r#type: FieldType<'a>,
    documentation: Option<Documentation<'a>>,
    updated_at: Option<FieldAttribute<'a>>,
    unique: Option<FieldAttribute<'a>>,
    id: Option<IdFieldDefinition<'a>>,
    default: Option<DefaultValue<'a>>,
    map: Option<FieldAttribute<'a>>,
    relation: Option<Relation<'a>>,
    native_type: Option<FieldAttribute<'a>>,
    ignore: Option<FieldAttribute<'a>>,
}

impl<'a> ModelField<'a> {
    /// Create a new required model field declaration.
    ///
    /// ```ignore
    /// model User {
    ///   name String
    /// //     ^^^^^^ type_name
    /// //^^^^ name
    /// }
    /// ```
    pub fn new_required(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::required(type_name))
    }

    /// Create a new optional model field declaration.
    ///
    /// ```ignore
    /// model Address {
    ///   street String?
    /// //       ^^^^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_optional(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::optional(type_name))
    }

    /// Create a new array model field declaration.
    ///
    /// ```ignore
    /// model Address {
    ///   street String[]
    /// //       ^^^^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_array(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::array(type_name))
    }

    /// Create a new required unsupported model field declaration.
    ///
    /// ```ignore
    /// model Address {
    ///   street Unsupported("foo")
    /// //                    ^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_required_unsupported(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::required_unsupported(type_name))
    }

    /// Create a new optional unsupported model field declaration.
    ///
    /// ```ignore
    /// model Address {
    ///   street Unsupported("foo")?
    /// //                    ^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_optional_unsupported(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::optional_unsupported(type_name))
    }

    /// Create a new array unsupported model field declaration.
    ///
    /// ```ignore
    /// model Address {
    ///   street Unsupported("foo")[]
    /// //                    ^^^ type_name
    /// //^^^^^^ name
    /// }
    /// ```
    pub fn new_array_unsupported(name: &'a str, type_name: &'a str) -> Self {
        Self::new(name, FieldType::array_unsupported(type_name))
    }

    /// Sets the field map attribute.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @map("Straße")
    ///                       ^^^^^^ value
    /// }
    /// ```
    pub fn map(&mut self, value: &'a str) {
        let mut map = Function::new("map");
        map.push_param(value);

        self.map = Some(FieldAttribute::new(map));
    }

    /// Documentation of the field.
    ///
    /// ```ignore
    /// model Foo {
    ///   /// This is the documentation.
    ///   bar Int
    /// }
    /// ```
    pub fn documentation(&mut self, documentation: impl Into<Cow<'a, str>>) {
        self.documentation = Some(Documentation(documentation.into()));
    }

    /// Sets the field default attribute.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @default("Prenzlauer Allee")
    ///                           ^^^^^^^^^^^^^^^^ value
    /// }
    /// ```
    pub fn default(&mut self, value: DefaultValue<'a>) {
        self.default = Some(value);
    }

    /// Sets the native type of the field.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @db.VarChar(255)
    /// //                          ^^^ param
    /// //                  ^^^^^^^ type_name
    /// //               ^^ prefix
    /// }
    /// ```
    ///
    /// TODO: `params` as `&[&str]` when we get rid of the DML.
    pub fn native_type(&mut self, prefix: &'a str, r#type: &'a str, params: Vec<String>) {
        let mut native_type = FieldAttribute::new(Function::new(r#type));

        for param in params {
            native_type.push_param(Constant::new_no_validate(param));
        }

        native_type.prefix(prefix);

        self.native_type = Some(native_type);
    }

    /// Marks the field to hold the update timestamp.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @updatedAt
    /// //              ^^^^^^^^^^ adds this
    /// }
    /// ```
    pub fn updated_at(&mut self) {
        self.updated_at = Some(FieldAttribute::new(Function::new("updatedAt")));
    }

    /// Marks the field to hold a unique constraint.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @unique(sort: Asc, length: 11)
    /// //              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn unique(&mut self, options: IndexFieldOptions<'a>) {
        let mut fun = Function::new("unique");

        if let Some(map) = options.map {
            fun.push_param(("map", Text::new(map)));
        }

        if let Some(sort_order) = options.sort_order {
            fun.push_param(("sort", Constant::new_no_validate(sort_order)));
        }

        if let Some(length) = options.length {
            fun.push_param(("length", Constant::new_no_validate(length)));
        }

        if let Some(clustered) = options.clustered {
            fun.push_param(("clustered", Constant::new_no_validate(clustered)));
        }

        self.unique = Some(FieldAttribute::new(fun));
    }

    /// Marks the field to be the id of the model.
    ///
    /// ```ignore
    /// model Address {
    ///   street String @id
    /// //              ^^^ this
    /// }
    /// ```
    pub fn id(&mut self, definition: IdFieldDefinition<'a>) {
        self.id = Some(definition);
    }

    /// Set the field to be a relation.
    ///
    /// ```ignore
    /// model Address {
    ///   street Street @relation(...)
    /// //              ^^^^^^^^^^^^^^ this
    /// }
    /// ```
    pub fn relation(&mut self, relation: Relation<'a>) {
        self.relation = Some(relation);
    }

    /// Ignores the field.
    ///
    /// ```ignore
    /// model Address {
    ///   street Street @ignore
    /// //              ^^^^^^^ this
    /// }
    /// ```
    pub fn ignore(&mut self) {
        self.ignore = Some(FieldAttribute::new(Function::new("ignore")));
    }

    /// Comments the field out.
    pub fn commented_out(&mut self) {
        self.commented_out = true;
    }

    fn new(name: &'a str, r#type: FieldType<'a>) -> Self {
        let name = Constant::new_no_validate(Cow::Borrowed(name));

        Self {
            name,
            commented_out: false,
            r#type,
            map: None,
            documentation: None,
            updated_at: None,
            unique: None,
            id: None,
            default: None,
            relation: None,
            native_type: None,
            ignore: None,
        }
    }

    /// Generate a model field rendering from the deprecated DML structure.
    ///
    /// Remove when destroying the DML. This API cannot really be
    /// public, because we need info from the model and it doesn't
    /// make much sense to call this from outside of the module.
    pub(super) fn from_dml(
        datasource: &'a psl::Datasource,
        _dml_model: &dml::Model,
        dml_field: &'a dml::Field,
        uniques: &HashMap<&'a str, IndexFieldOptions<'a>>,
        id: Option<IdFieldDefinition<'a>>,
    ) -> Self {
        match dml_field {
            dml::Field::ScalarField(ref sf) => {
                let (r#type, native_type) = match sf.field_type {
                    dml::FieldType::Enum(ref ct) => (ct.as_str(), None),
                    dml::FieldType::Relation(ref info) => (info.referenced_model.as_str(), None),
                    dml::FieldType::Unsupported(ref s) => (s.as_str(), None),
                    dml::FieldType::Scalar(ref st, ref nt) => {
                        (st.as_ref(), nt.as_ref().map(|nt| (nt.name(), nt.args())))
                    }
                    dml::FieldType::CompositeType(ref ct) => (ct.as_str(), None),
                };

                let mut field = match sf.arity {
                    dml::FieldArity::Required if sf.field_type.is_unsupported() => {
                        Self::new_required_unsupported(&sf.name, r#type)
                    }
                    dml::FieldArity::Optional if sf.field_type.is_unsupported() => {
                        Self::new_optional_unsupported(&sf.name, r#type)
                    }
                    dml::FieldArity::List if sf.field_type.is_unsupported() => {
                        Self::new_array_unsupported(&sf.name, r#type)
                    }
                    dml::FieldArity::Required => Self::new_required(&sf.name, r#type),
                    dml::FieldArity::Optional => Self::new_optional(&sf.name, r#type),
                    dml::FieldArity::List => Self::new_array(&sf.name, r#type),
                };

                if let Some(ref docs) = sf.documentation {
                    field.documentation(docs);
                }

                if let Some(dv) = sf.default_value() {
                    field.default(DefaultValue::from_dml(dv));
                }

                if let Some((name, args)) = native_type {
                    field.native_type(&datasource.name, name, args);
                }

                if sf.is_updated_at {
                    field.updated_at();
                }

                if let Some(unique) = uniques.get(sf.name.as_str()) {
                    field.unique(*unique);
                }

                if sf.is_ignored {
                    field.ignore();
                }

                if sf.is_commented_out {
                    field.commented_out();
                }

                if let Some(ref map) = sf.database_name {
                    field.map(map);
                }

                if let Some(id) = id {
                    field.id(id);
                }

                field
            }
            dml::Field::RelationField(rf) => {
                let mut field = match rf.arity {
                    dml::FieldArity::Required => Self::new_required(&rf.name, &rf.relation_info.referenced_model),
                    dml::FieldArity::Optional => Self::new_optional(&rf.name, &rf.relation_info.referenced_model),
                    dml::FieldArity::List => Self::new_array(&rf.name, &rf.relation_info.referenced_model),
                };

                if let Some(ref docs) = rf.documentation {
                    field.documentation(docs);
                }

                if rf.is_commented_out {
                    field.commented_out();
                }

                if rf.is_ignored {
                    field.ignore();
                }

                let dml_info = &rf.relation_info;
                let relation_name = dml_info.name.as_str();

                // :(
                if !relation_name.is_empty() || (!dml_info.fields.is_empty() || !dml_info.references.is_empty()) {
                    let mut relation = Relation::new();

                    if !relation_name.is_empty() {
                        relation.name(relation_name);
                    }

                    relation.fields(dml_info.fields.iter().map(AsRef::as_ref));
                    relation.references(dml_info.references.iter().map(AsRef::as_ref));

                    if let Some(ref action) = dml_info.on_delete {
                        relation.on_delete(action.as_ref());
                    }

                    if let Some(ref action) = dml_info.on_update {
                        relation.on_update(action.as_ref());
                    }

                    if let Some(ref map) = &dml_info.fk_name {
                        relation.map(map);
                    }

                    field.relation(relation);
                }

                field
            }
            dml::Field::CompositeField(cf) => {
                let mut field = match cf.arity {
                    dml::FieldArity::Required => Self::new_required(&cf.name, &cf.composite_type),
                    dml::FieldArity::Optional => Self::new_optional(&cf.name, &cf.composite_type),
                    dml::FieldArity::List => Self::new_array(&cf.name, &cf.composite_type),
                };

                if let Some(ref docs) = cf.documentation {
                    field.documentation(docs);
                }

                if let Some(ref map) = cf.database_name {
                    field.map(map);
                }

                if cf.is_commented_out {
                    field.commented_out();
                }

                if cf.is_ignored {
                    field.ignore();
                }

                if let Some(ref dv) = cf.default_value {
                    field.default(DefaultValue::from_dml(dv));
                }

                field
            }
        }
    }
}

impl<'a> fmt::Display for ModelField<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref docs) = self.documentation {
            docs.fmt(f)?;
        }

        if self.commented_out {
            f.write_str("// ")?;
        }

        write!(f, "{} {}", self.name, self.r#type)?;

        if let Some(ref updated_at) = self.updated_at {
            write!(f, " {updated_at}")?;
        }

        if let Some(ref unique) = self.unique {
            write!(f, " {unique}")?;
        }

        if let Some(ref id) = self.id {
            write!(f, " {id}")?;
        }

        if let Some(ref def) = self.default {
            write!(f, " {def}")?;
        }

        if let Some(ref map) = self.map {
            write!(f, " {map}")?;
        }

        if let Some(ref relation) = self.relation {
            write!(f, " {relation}")?;
        }

        if let Some(ref nt) = self.native_type {
            write!(f, " {nt}")?;
        }

        if let Some(ref ignore) = self.ignore {
            write!(f, " {ignore}")?;
        }

        Ok(())
    }
}
