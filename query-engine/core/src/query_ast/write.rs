//! Write query AST
use super::FilteredQuery;
use connector::filter::Filter;
use prisma_models::prelude::*;

#[derive(Debug, Clone)]
pub enum WriteQuery {
    CreateRecord(CreateRecord),
    UpdateRecord(UpdateRecord),
    DeleteRecord(DeleteRecord),
    UpdateManyRecords(UpdateManyRecords),
    DeleteManyRecords(DeleteManyRecords),
    ConnectRecords(ConnectRecords),
    DisconnectRecords(DisconnectRecords),
    ResetData(ResetData),
}

impl WriteQuery {
    pub fn inject_inlined_identifier(&mut self, id: RecordIdentifier) {
        id.into_iter()
            .for_each(|(field, value)| self.inject_args(field, vec![value]));
    }

    // Injects PrismaValues into the write arguments based the passed field.
    // If the underlying representation of the field takes multiple values,
    pub fn inject_args(&mut self, field: Field, values: Vec<PrismaValue>) {
        match self {
            Self::CreateRecord(x) => {
                x.args.insert(key, value);
            }

            Self::UpdateRecord(x) => {
                x.args.insert(key, value);
            }

            Self::UpdateManyRecords(x) => {
                x.args.insert(key, value);
            }

            _ => (),
        };
    }
}

impl FilteredQuery for WriteQuery {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        match self {
            Self::UpdateRecord(q) => q.get_filter(),
            Self::DeleteManyRecords(q) => q.get_filter(),
            Self::DeleteRecord(q) => q.get_filter(),
            Self::UpdateManyRecords(q) => q.get_filter(),
            _ => unimplemented!(),
        }
    }

    fn set_filter(&mut self, filter: Filter) {
        match self {
            Self::UpdateRecord(q) => q.set_filter(filter),
            Self::DeleteManyRecords(q) => q.set_filter(filter),
            Self::DeleteRecord(q) => q.set_filter(filter),
            Self::UpdateManyRecords(q) => q.set_filter(filter),
            _ => unimplemented!(),
        }
    }
}

impl std::fmt::Display for WriteQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::CreateRecord(q) => write!(f, "CreateRecord(model: {}, args: {:?})", q.model.name, q.args,),
            Self::UpdateRecord(q) => write!(
                f,
                "UpdateRecord(model: {}, finder: {:?}, args: {:?})",
                q.model.name, q.where_, q.args,
            ),
            Self::DeleteRecord(q) => write!(f, "DeleteRecord: {}, {:?}", q.model.name, q.where_),
            Self::UpdateManyRecords(q) => write!(f, "UpdateManyRecords(model: {}, args: {:?})", q.model.name, q.args),
            Self::DeleteManyRecords(q) => write!(f, "DeleteManyRecords: {}", q.model.name),
            Self::ConnectRecords(_) => write!(f, "ConnectRecords"),
            Self::DisconnectRecords(_) => write!(f, "DisconnectRecords"),
            Self::ResetData(_) => write!(f, "ResetData"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateRecord {
    pub model: ModelRef,
    pub args: PrismaArgs,
}

#[derive(Debug, Clone)]
pub struct UpdateRecord {
    pub model: ModelRef,
    pub where_: Filter,
    pub args: PrismaArgs,
}

#[derive(Debug, Clone)]
pub struct UpdateManyRecords {
    pub model: ModelRef,
    pub filter: Filter,
    pub args: PrismaArgs,
}

#[derive(Debug, Clone)]
pub struct DeleteRecord {
    pub model: ModelRef,
    pub where_: Option<Filter>,
}

#[derive(Debug, Clone)]
pub struct DeleteManyRecords {
    pub model: ModelRef,
    pub filter: Filter,
}

#[derive(Debug, Clone)]
pub struct ConnectRecords {
    pub parent_id: Option<RecordIdentifier>,
    pub child_ids: Vec<RecordIdentifier>,
    pub relation_field: RelationFieldRef,
}

#[derive(Debug, Clone)]
pub struct DisconnectRecords {
    pub parent_id: Option<RecordIdentifier>,
    pub child_ids: Vec<RecordIdentifier>,
    pub relation_field: RelationFieldRef,
}

#[derive(Debug, Clone)]
pub struct ResetData {
    pub internal_data_model: InternalDataModelRef,
}

impl FilteredQuery for UpdateRecord {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        Some(&mut self.where_)
    }

    fn set_filter(&mut self, filter: Filter) {
        self.where_ = filter
    }
}

impl FilteredQuery for UpdateManyRecords {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        Some(&mut self.filter)
    }

    fn set_filter(&mut self, filter: Filter) {
        self.filter = filter
    }
}

impl FilteredQuery for DeleteManyRecords {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        Some(&mut self.filter)
    }

    fn set_filter(&mut self, filter: Filter) {
        self.filter = filter
    }
}

impl FilteredQuery for DeleteRecord {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        self.where_.as_mut()
    }

    fn set_filter(&mut self, filter: Filter) {
        self.where_ = Some(filter)
    }
}
