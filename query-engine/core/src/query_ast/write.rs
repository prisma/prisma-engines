//! Write query AST
use super::{FilteredNestedMutation, FilteredQuery};
use crate::{RecordQuery, ToGraphviz};
use connector::{filter::Filter, DatasourceFieldName, NativeUpsert, RecordFilter, WriteArgs};
use prisma_models::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) enum WriteQuery {
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
    Upsert(NativeUpsert),
}

impl WriteQuery {
    /// Takes a SelectionResult and writes its contents into the write arguments of the underlying query.
    pub fn inject_result_into_args(&mut self, result: SelectionResult) {
        let model = self.model();
        let args = match self {
            Self::CreateRecord(ref mut x) => &mut x.args,
            Self::UpdateRecord(x) => &mut x.args,
            Self::UpdateManyRecords(x) => &mut x.args,
            _ => return,
        };

        for (selected_field, value) in result {
            args.insert(
                DatasourceFieldName(selected_field.db_name().to_owned()),
                (&selected_field, value),
            )
        }

        args.update_datetimes(&model);
    }

    pub fn set_selectors(&mut self, selectors: Vec<SelectionResult>) {
        match self {
            Self::UpdateManyRecords(x) => x.set_selectors(selectors),
            Self::UpdateRecord(x) => x.set_selectors(selectors),
            Self::DeleteRecord(x) => x.set_selectors(selectors),
            _ => (),
        }
    }

    pub fn returns(&self, field_selection: &FieldSelection) -> bool {
        let returns_id = &self.model().primary_identifier() == field_selection;

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
            Self::Upsert(_) => returns_id,
        }
    }

    pub fn model(&self) -> Model {
        match self {
            Self::CreateRecord(q) => q.model.clone(),
            Self::CreateManyRecords(q) => q.model.clone(),
            Self::UpdateRecord(q) => q.model.clone(),
            Self::Upsert(q) => q.model().clone(),
            Self::DeleteRecord(q) => q.model.clone(),
            Self::UpdateManyRecords(q) => q.model.clone(),
            Self::DeleteManyRecords(q) => q.model.clone(),
            Self::ConnectRecords(q) => q.relation_field.model(),
            Self::DisconnectRecords(q) => q.relation_field.model(),
            Self::ExecuteRaw(_) => unimplemented!(),
            Self::QueryRaw(_) => unimplemented!(),
        }
    }

    pub fn native_upsert(
        name: String,
        model: Model,
        record_filter: RecordFilter,
        create: WriteArgs,
        update: WriteArgs,
        read: RecordQuery,
    ) -> crate::Query {
        crate::Query::Write(WriteQuery::Upsert(NativeUpsert::new(
            name,
            model,
            record_filter,
            create,
            update,
            read.selected_fields,
            read.selection_order,
        )))
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateRecord(q) => write!(f, "CreateRecord(model: {}, args: {:?})", q.model.name(), q.args),
            Self::CreateManyRecords(q) => write!(f, "CreateManyRecord(model: {})", q.model.name()),
            Self::UpdateRecord(q) => write!(
                f,
                "UpdateRecord(model: {}, filter: {:?}, args: {:?})",
                q.model.name(),
                q.record_filter,
                q.args,
            ),
            Self::DeleteRecord(q) => write!(f, "DeleteRecord: {}, {:?}", q.model.name(), q.record_filter),
            Self::UpdateManyRecords(q) => write!(f, "UpdateManyRecords(model: {}, args: {:?})", q.model.name(), q.args),
            Self::DeleteManyRecords(q) => write!(f, "DeleteManyRecords: {}", q.model.name()),
            Self::ConnectRecords(_) => write!(f, "ConnectRecords"),
            Self::DisconnectRecords(_) => write!(f, "DisconnectRecords"),
            Self::ExecuteRaw(r) => write!(f, "ExecuteRaw: {:?}", r.inputs),
            Self::QueryRaw(r) => write!(f, "QueryRaw: {:?}", r.inputs),
            Self::Upsert(q) => write!(
                f,
                "Upsert(model: {}, filter: {:?}, create: {:?}, update: {:?}",
                q.model().name(),
                q.record_filter(),
                q.create(),
                q.update()
            ),
        }
    }
}

impl ToGraphviz for WriteQuery {
    fn to_graphviz(&self) -> String {
        match self {
            Self::CreateRecord(q) => format!("CreateRecord(model: {}, args: {:?})", q.model.name(), q.args),
            Self::CreateManyRecords(q) => format!("CreateManyRecord(model: {})", q.model.name()),
            Self::UpdateRecord(q) => format!("UpdateRecord(model: {})", q.model.name(),),
            Self::DeleteRecord(q) => format!("DeleteRecord: {}, {:?}", q.model.name(), q.record_filter),
            Self::UpdateManyRecords(q) => format!("UpdateManyRecords(model: {}, args: {:?})", q.model.name(), q.args),
            Self::DeleteManyRecords(q) => format!("DeleteManyRecords: {}", q.model.name()),
            Self::ConnectRecords(_) => "ConnectRecords".to_string(),
            Self::DisconnectRecords(_) => "DisconnectRecords".to_string(),
            Self::ExecuteRaw(r) => format!("ExecuteRaw: {:?}", r.inputs),
            Self::QueryRaw(r) => format!("QueryRaw: {:?}", r.inputs),
            Self::Upsert(q) => format!("Upsert(model: {}", q.model().name()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateRecord {
    pub name: String,
    pub model: Model,
    pub args: WriteArgs,
    pub selected_fields: FieldSelection,
    pub selection_order: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreateManyRecords {
    pub model: Model,
    pub args: Vec<WriteArgs>,
    pub skip_duplicates: bool,
}

impl CreateManyRecords {
    pub fn inject_result_into_all(&mut self, result: SelectionResult) {
        for (selected_field, value) in result {
            for args in self.args.iter_mut() {
                args.insert(
                    DatasourceFieldName(selected_field.db_name().to_owned()),
                    (&selected_field, value.clone()),
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateRecord {
    pub model: Model,
    pub record_filter: RecordFilter,
    pub args: WriteArgs,
}

#[derive(Debug, Clone)]
pub struct UpdateManyRecords {
    pub model: Model,
    pub record_filter: RecordFilter,
    pub args: WriteArgs,
}

#[derive(Debug, Clone)]
pub struct DeleteRecord {
    pub model: Model,
    pub record_filter: Option<RecordFilter>,
}

#[derive(Debug, Clone)]
pub struct DeleteManyRecords {
    pub model: Model,
    pub record_filter: RecordFilter,
}

#[derive(Debug, Clone)]
pub struct ConnectRecords {
    pub parent_id: Option<SelectionResult>,
    pub child_ids: Vec<SelectionResult>,
    pub relation_field: RelationFieldRef,
}

#[derive(Debug, Clone)]
pub struct DisconnectRecords {
    pub parent_id: Option<SelectionResult>,
    pub child_ids: Vec<SelectionResult>,
    pub relation_field: RelationFieldRef,
}

#[derive(Debug, Clone)]
pub struct RawQuery {
    /// Model associated with the raw query, if one is necessary
    pub model: Option<Model>,
    /// Map of query arguments and their values
    pub inputs: HashMap<String, PrismaValue>,
    /// Hint as to what kind of query is being executed
    pub query_type: Option<String>,
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
    }
}

impl FilteredNestedMutation for UpdateRecord {
    fn set_selectors(&mut self, selectors: Vec<SelectionResult>) {
        self.record_filter.selectors = Some(selectors);
    }
}

impl FilteredNestedMutation for UpdateManyRecords {
    fn set_selectors(&mut self, selectors: Vec<SelectionResult>) {
        self.record_filter.selectors = Some(selectors);
    }
}

impl FilteredNestedMutation for DeleteRecord {
    fn set_selectors(&mut self, selectors: Vec<SelectionResult>) {
        if let Some(ref mut rf) = self.record_filter {
            rf.selectors = Some(selectors);
        } else {
            self.record_filter = Some(selectors.into())
        }
    }
}
