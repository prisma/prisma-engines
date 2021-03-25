//! Write query AST
use super::FilteredQuery;
use connector::{filter::Filter, DatasourceFieldName, RecordFilter, WriteArgs};
use prisma_models::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum WriteQuery {
    CreateRecord(CreateRecord),
    CreateManyRecords(CreateManyRecords),
    UpdateRecord(UpdateRecord),
    DeleteRecord(DeleteRecord),
    UpdateManyRecords(UpdateManyRecords),
    DeleteManyRecords(DeleteManyRecords),
    ConnectRecords(ConnectRecords),
    DisconnectRecords(DisconnectRecords),
    ExecuteRaw(RawQuery),
    QueryRaw(RawQuery),
}

impl WriteQuery {
    #[tracing::instrument(skip(self, projection))]
    pub fn inject_projection_into_args(&mut self, projection: RecordProjection) {
        let keys: Vec<_> = projection.fields().map(|sf| sf.db_name().to_owned()).collect();
        let values: Vec<_> = projection.values().collect();

        let args = match self {
            Self::CreateRecord(ref mut x) => &mut x.args,
            Self::UpdateRecord(x) => &mut x.args,
            Self::UpdateManyRecords(x) => &mut x.args,
            _ => return,
        };

        let model = projection.model().expect("Model was not found");

        keys.into_iter()
            .zip(values)
            .for_each(|(key, value)| args.insert(DatasourceFieldName(key), value));

        args.update_datetimes(model);
    }

    #[tracing::instrument(skip(self, projection))]
    pub fn returns(&self, projection: &ModelProjection) -> bool {
        let returns_id = &self.model().primary_identifier() == projection;

        // Write operations only return IDs at the moment, so anything different
        // from the primary ID is automatically not returned.
        // DeleteMany, Connect and Disconnect do not return anything.
        match self {
            Self::CreateRecord(_) => returns_id,
            Self::CreateManyRecords(_) => false,
            Self::UpdateRecord(_) => returns_id,
            Self::DeleteRecord(_) => returns_id,
            Self::UpdateManyRecords(_) => returns_id,
            Self::DeleteManyRecords(_) => false,
            Self::ConnectRecords(_) => false,
            Self::DisconnectRecords(_) => false,
            Self::ExecuteRaw(_) => false,
            Self::QueryRaw(_) => false,
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn model(&self) -> ModelRef {
        match self {
            Self::CreateRecord(q) => Arc::clone(&q.model),
            Self::CreateManyRecords(q) => Arc::clone(&q.model),
            Self::UpdateRecord(q) => Arc::clone(&q.model),
            Self::DeleteRecord(q) => Arc::clone(&q.model),
            Self::UpdateManyRecords(q) => Arc::clone(&q.model),
            Self::DeleteManyRecords(q) => Arc::clone(&q.model),
            Self::ConnectRecords(q) => q.relation_field.model(),
            Self::DisconnectRecords(q) => q.relation_field.model(),
            Self::ExecuteRaw(_) => unimplemented!(),
            Self::QueryRaw(_) => unimplemented!(),
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
            Self::CreateRecord(q) => write!(f, "CreateRecord(model: {}, args: {:?})", q.model.name, q.args),
            Self::CreateManyRecords(q) => write!(f, "CreateManyRecord(model: {})", q.model.name),
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
            Self::ExecuteRaw(r) => write!(f, "ExecuteRaw: {} ({:?})", r.query, r.parameters),
            Self::QueryRaw(r) => write!(f, "QueryRaw: {} ({:?})", r.query, r.parameters),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateRecord {
    pub model: ModelRef,
    pub args: WriteArgs,
}

#[derive(Debug, Clone)]
pub struct CreateManyRecords {
    pub model: ModelRef,
    pub args: Vec<WriteArgs>,
    pub skip_duplicates: bool,
}

impl CreateManyRecords {
    #[tracing::instrument(skip(self, projection))]
    pub fn inject_all(&mut self, projection: RecordProjection) {
        let keys: Vec<_> = projection.fields().map(|sf| sf.db_name().to_owned()).collect();
        let values: Vec<_> = projection.values().collect();

        let zipped = keys.into_iter().zip(values);

        for arg in self.args.iter_mut() {
            zipped
                .clone()
                .for_each(|(key, value)| arg.insert(DatasourceFieldName(key), value));
        }
    }
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

#[derive(Debug, Clone)]
pub struct RawQuery {
    pub query: String,
    pub parameters: Vec<PrismaValue>,
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
