//! Write query AST
use super::FilteredQuery;
use crate::RawQueryType;
use connector::{filter::Filter, DatasourceFieldName, RecordFilter, WriteArgs};
use prisma_models::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum WriteQuery {
    CreateRecord(CreateRecord),
    UpdateRecord(UpdateRecord),
    DeleteRecord(DeleteRecord),
    UpdateManyRecords(UpdateManyRecords),
    DeleteManyRecords(DeleteManyRecords),
    ConnectRecords(ConnectRecords),
    DisconnectRecords(DisconnectRecords),
    Raw {
        query: String,
        parameters: Vec<PrismaValue>,
        raw_type: RawQueryType,
    },
}

impl WriteQuery {
    pub fn inject_projection_into_args(&mut self, projection: RecordProjection) {
        let keys = projection.fields().map(|sf| sf.db_name().to_owned()).collect();
        let values = projection.values().map(|v| v.clone()).collect();

        self.inject_values_into_args(keys, values);
    }

    pub fn inject_values_into_args(&mut self, keys: Vec<String>, values: Vec<PrismaValue>) {
        keys.into_iter()
            .zip(values)
            .for_each(|(key, value)| self.inject_field_arg(key, value));
    }

    // Injects PrismaValues into the write arguments based the passed key.
    pub fn inject_field_arg(&mut self, key: String, value: PrismaValue) {
        let args = match self {
            Self::CreateRecord(ref mut x) => &mut x.args,
            Self::UpdateRecord(x) => &mut x.args,
            Self::UpdateManyRecords(x) => &mut x.args,

            _ => return,
        };

        args.insert(DatasourceFieldName(key), value)
    }

    pub fn returns(&self, projection: &ModelProjection) -> bool {
        let returns_id = &self.model().primary_identifier() == projection;

        // Write operations only return IDs at the moment, so anything different
        // from the primary ID is automatically not returned.
        // DeleteMany, Connect and Disconnect do not return anything.
        match self {
            Self::CreateRecord(_) => returns_id,
            Self::UpdateRecord(_) => returns_id,
            Self::DeleteRecord(_) => returns_id,
            Self::UpdateManyRecords(_) => returns_id,
            Self::DeleteManyRecords(_) => false,
            Self::ConnectRecords(_) => false,
            Self::DisconnectRecords(_) => false,
            Self::Raw { .. } => unimplemented!(),
        }
    }

    pub fn model(&self) -> ModelRef {
        match self {
            Self::CreateRecord(q) => Arc::clone(&q.model),
            Self::UpdateRecord(q) => Arc::clone(&q.model),
            Self::DeleteRecord(q) => Arc::clone(&q.model),
            Self::UpdateManyRecords(q) => Arc::clone(&q.model),
            Self::DeleteManyRecords(q) => Arc::clone(&q.model),
            Self::ConnectRecords(q) => q.relation_field.model(),
            Self::DisconnectRecords(q) => q.relation_field.model(),
            Self::Raw { .. } => unimplemented!(),
        }
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
                "UpdateRecord(model: {}, filter: {:?}, args: {:?})",
                q.model.name, q.record_filter, q.args,
            ),
            Self::DeleteRecord(q) => write!(f, "DeleteRecord: {}, {:?}", q.model.name, q.record_filter),
            Self::UpdateManyRecords(q) => write!(f, "UpdateManyRecords(model: {}, args: {:?})", q.model.name, q.args),
            Self::DeleteManyRecords(q) => write!(f, "DeleteManyRecords: {}", q.model.name),
            Self::ConnectRecords(_) => write!(f, "ConnectRecords"),
            Self::DisconnectRecords(_) => write!(f, "DisconnectRecords"),
            Self::Raw {
                query,
                parameters,
                raw_type,
            } => write!(f, "Raw ({:?}): {} ({:?})", raw_type, query, parameters),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateRecord {
    pub model: ModelRef,
    pub args: WriteArgs,
}

#[derive(Debug, Clone)]
pub struct UpdateRecord {
    pub model: ModelRef,
    pub record_filter: RecordFilter,
    pub args: WriteArgs,
}

#[derive(Debug, Clone)]
pub struct UpdateManyRecords {
    pub model: ModelRef,
    pub record_filter: RecordFilter,
    pub args: WriteArgs,
}

#[derive(Debug, Clone)]
pub struct DeleteRecord {
    pub model: ModelRef,
    pub record_filter: Option<RecordFilter>,
}

#[derive(Debug, Clone)]
pub struct DeleteManyRecords {
    pub model: ModelRef,
    pub record_filter: RecordFilter,
}

#[derive(Debug, Clone)]
pub struct ConnectRecords {
    pub parent_id: Option<RecordProjection>,
    pub child_ids: Vec<RecordProjection>,
    pub relation_field: RelationFieldRef,
}

#[derive(Debug, Clone)]
pub struct DisconnectRecords {
    pub parent_id: Option<RecordProjection>,
    pub child_ids: Vec<RecordProjection>,
    pub relation_field: RelationFieldRef,
}

impl FilteredQuery for UpdateRecord {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        Some(&mut self.record_filter.filter)
    }

    fn set_filter(&mut self, filter: Filter) {
        self.record_filter.filter = filter
    }
}

impl FilteredQuery for UpdateManyRecords {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        Some(&mut self.record_filter.filter)
    }

    fn set_filter(&mut self, filter: Filter) {
        self.record_filter.filter = filter
    }
}

impl FilteredQuery for DeleteManyRecords {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        Some(&mut self.record_filter.filter)
    }

    fn set_filter(&mut self, filter: Filter) {
        self.record_filter.filter = filter
    }
}

impl FilteredQuery for DeleteRecord {
    fn get_filter(&mut self) -> Option<&mut Filter> {
        self.record_filter.as_mut().map(|f| &mut f.filter)
    }

    fn set_filter(&mut self, filter: Filter) {
        match self.record_filter {
            Some(ref mut rf) => rf.filter = filter,
            None => self.record_filter = Some(filter.into()),
        }

        //.filter = Some(filter)
    }
}
