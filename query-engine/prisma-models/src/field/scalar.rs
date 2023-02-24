use crate::{ast, parent_container::ParentContainer, prelude::*};
use dml::{FieldArity, NativeTypeInstance};
use psl::parser_database::{walkers, ScalarFieldType};
use std::fmt::{Debug, Display};

pub type ScalarField = crate::Zipper<ScalarFieldId>;
pub type ScalarFieldRef = ScalarField;
pub type ScalarFieldWeak = ScalarField;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ScalarFieldId {
    InModel(walkers::ScalarFieldId),
    InCompositeType((ast::CompositeTypeId, ast::FieldId)),
}

impl ScalarField {
    pub fn is_id(&self) -> bool {
        match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).is_single_pk(),
            ScalarFieldId::InCompositeType(_) => false,
        }
    }

    pub fn is_list(&self) -> bool {
        matches!(self.arity(), FieldArity::List)
    }

    pub fn is_required(&self) -> bool {
        matches!(self.arity(), FieldArity::Required)
    }

    pub fn unique(&self) -> bool {
        match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).is_unique(),
            ScalarFieldId::InCompositeType(_) => false, // TODO: is this right?
        }
    }

    pub fn db_name(&self) -> &str {
        match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).database_name(),
            ScalarFieldId::InCompositeType(id) => self.dm.walk(id).database_name(),
        }
    }

    pub fn type_identifier_with_arity(&self) -> (TypeIdentifier, FieldArity) {
        (self.type_identifier(), self.arity())
    }

    pub fn is_read_only(&self) -> bool {
        let sfid = match self.id {
            ScalarFieldId::InModel(id) => id,
            ScalarFieldId::InCompositeType(_) => return false,
        };
        let sf = self.dm.walk(sfid);
        let mut relation_fields = sf.model().relation_fields();
        relation_fields.any(|rf| rf.fields().into_iter().flatten().any(|sf2| sf.id == sf2.id))
    }

    pub fn is_numeric(&self) -> bool {
        self.type_identifier().is_numeric()
    }

    pub fn container(&self) -> ParentContainer {
        match self.id {
            ScalarFieldId::InModel(id) => self.dm.find_model_by_id(self.dm.walk(id).model().id).into(),
            ScalarFieldId::InCompositeType((id, _)) => self.dm.find_composite_type_by_id(id).into(),
        }
    }

    pub fn name(&self) -> &str {
        match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).name(),
            ScalarFieldId::InCompositeType(id) => self.dm.walk(id).name(),
        }
    }

    pub fn type_identifier(&self) -> TypeIdentifier {
        let scalar_field_type = match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).scalar_field_type(),
            ScalarFieldId::InCompositeType(id) => self.dm.walk(id).r#type(),
        };

        match scalar_field_type {
            ScalarFieldType::CompositeType(_) => unreachable!(),
            ScalarFieldType::Enum(x) => TypeIdentifier::Enum(x),
            ScalarFieldType::BuiltInScalar(scalar) => scalar.into(),
            ScalarFieldType::Unsupported(_) => TypeIdentifier::Unsupported,
        }
    }

    pub fn arity(&self) -> FieldArity {
        match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).ast_field().arity,
            ScalarFieldId::InCompositeType(id) => self.dm.walk(id).arity(),
        }
    }

    pub fn internal_enum(&self) -> Option<crate::InternalEnum> {
        let enum_id = match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).scalar_field_type().as_enum(),
            ScalarFieldId::InCompositeType(id) => self.dm.walk(id).r#type().as_enum(),
        }?;
        Some(self.dm.clone().zip(enum_id))
    }

    pub fn default_value(&self) -> Option<dml::DefaultKind> {
        match self.id {
            ScalarFieldId::InModel(id) => {
                let walker = self.dm.walk(id);
                walker.default_value().map(|dv| {
                    dml::dml_default_kind(&dv.ast_attribute().arguments.arguments[0].value, walker.scalar_type())
                })
            }
            ScalarFieldId::InCompositeType(id) => {
                let walker = self.dm.walk(id);
                walker
                    .default_value()
                    .map(|dv| dml::dml_default_kind(dv, walker.scalar_type()))
            }
        }
    }

    pub fn is_updated_at(&self) -> bool {
        match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).is_updated_at(),
            ScalarFieldId::InCompositeType(_) => false,
        }
    }

    pub fn is_auto_generated_int_id(&self) -> bool {
        match self.id {
            ScalarFieldId::InModel(id) => {
                let walker = self.dm.walk(id);
                walker.is_single_pk()
                    && matches!(
                        walker.default_value().map(|v| v.value()),
                        Some(ast::Expression::Function(name, _, _)) if name == "autoincrement" || name == "sequence"
                    )
                    && matches!(walker.scalar_type(), Some(psl::parser_database::ScalarType::Int))
            }
            ScalarFieldId::InCompositeType(_) => false,
        }
    }

    pub fn native_type(&self) -> Option<NativeTypeInstance> {
        let (_, name, args, span) = match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).raw_native_type(),
            ScalarFieldId::InCompositeType(id) => self.dm.walk(id).raw_native_type(),
        }?;
        let connector = self.dm.schema.connector;

        let nt = connector
            .parse_native_type(name, args, span, &mut Default::default())
            .unwrap();

        Some(dml::NativeTypeInstance {
            native_type: nt,
            connector,
        })
    }

    pub fn is_autoincrement(&self) -> bool {
        match self.id {
            ScalarFieldId::InModel(id) => self.dm.walk(id).is_autoincrement(),
            ScalarFieldId::InCompositeType(_) => false,
        }
    }
}

impl Display for ScalarField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.container().name(), self.name())
    }
}

impl From<(InternalDataModelRef, walkers::IndexFieldWalker<'_>)> for ScalarFieldRef {
    fn from((dm, f): (InternalDataModelRef, walkers::IndexFieldWalker<'_>)) -> Self {
        match f {
            walkers::IndexFieldWalker::Scalar(sf) => dm.zip(ScalarFieldId::InModel(sf.id)),
            walkers::IndexFieldWalker::Composite(cf) => dm.zip(ScalarFieldId::InCompositeType(cf.id)),
        }
    }
}
