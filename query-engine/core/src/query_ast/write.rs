//! Write query AST
use super::{FilteredNestedMutation, FilteredQuery};
use crate::{ReadQuery, RecordQuery, ToGraphviz};
use connector::{DatasourceFieldName, NativeUpsert, RecordFilter, WriteArgs};
use query_structure::{prelude::*, Filter};
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

        let inject = |args: &mut WriteArgs| {
            for (selected_field, value) in result.pairs() {
                args.insert(
                    DatasourceFieldName(selected_field.db_name().into_owned()),
                    (selected_field, value.clone()),
                )
            }
            args.update_datetimes(&model);
        };

        match self {
            Self::CreateRecord(ref mut x) => inject(&mut x.args),
            Self::CreateManyRecords(ref mut x) => x.args.iter_mut().map(inject).collect(),
            Self::UpdateRecord(ref mut x) => match x {
                UpdateRecord::WithSelection(u) => inject(&mut u.args),
                UpdateRecord::WithoutSelection(u) => inject(&mut u.args),
            },
            Self::UpdateManyRecords(x) => inject(&mut x.args),
            _ => (),
        };
    }

    pub fn set_selectors(&mut self, selectors: Vec<SelectionResult>) {
        match self {
            Self::UpdateManyRecords(x) => x.set_selectors(selectors),
            Self::UpdateRecord(x) => x.set_selectors(selectors),
            Self::DeleteRecord(x) => x.set_selectors(selectors),
            _ => (),
        }
    }

    /// Checks whether or not the field selection of this query satisfies the inputted field selection.
    pub fn satisfies(&self, field_selection: &FieldSelection) -> bool {
        self.returns()
            .map(|fs| fs.is_superset_of(field_selection))
            .unwrap_or(false)
    }

    /// Returns the field selection of a write query.
    ///
    /// Most write operations only return IDs at the moment, so anything different
    /// from the primary ID is automatically not returned.
    /// DeleteMany, Connect and Disconnect do not return anything.
    fn returns(&self) -> Option<FieldSelection> {
        let returns_id = Some(self.model().primary_identifier());

        match self {
            Self::CreateRecord(cr) => Some(cr.selected_fields.clone()),
            Self::CreateManyRecords(CreateManyRecords {
                selected_fields: Some(selected_fields),
                ..
            }) => Some(selected_fields.fields.clone()),
            Self::CreateManyRecords(CreateManyRecords {
                selected_fields: None, ..
            }) => None,
            Self::UpdateRecord(UpdateRecord::WithSelection(ur)) => Some(ur.selected_fields.clone()),
            Self::UpdateRecord(UpdateRecord::WithoutSelection(_)) => returns_id,
            Self::DeleteRecord(DeleteRecord {
                selected_fields: Some(selected_fields),
                ..
            }) => Some(selected_fields.fields.clone()),
            Self::DeleteRecord(_) => returns_id,
            Self::UpdateManyRecords(_) => returns_id,
            Self::DeleteManyRecords(_) => None,
            Self::ConnectRecords(_) => None,
            Self::DisconnectRecords(_) => None,
            Self::ExecuteRaw(_) => None,
            Self::QueryRaw(_) => None,
            Self::Upsert(_) => returns_id,
        }
    }

    /// Updates the field selection of the query to satisfy the inputted FieldSelection.
    pub fn satisfy_dependency(&mut self, fields: FieldSelection) {
        match self {
            Self::CreateRecord(cr) => cr.selected_fields.merge_in_place(fields),
            Self::UpdateRecord(UpdateRecord::WithSelection(ur)) => ur.selected_fields.merge_in_place(fields),
            Self::UpdateRecord(UpdateRecord::WithoutSelection(_)) => (),
            Self::CreateManyRecords(CreateManyRecords {
                selected_fields: Some(selected_fields),
                ..
            }) => selected_fields.fields.merge_in_place(fields),
            Self::CreateManyRecords(CreateManyRecords {
                selected_fields: None, ..
            }) => (),
            Self::DeleteRecord(DeleteRecord {
                selected_fields: Some(selected_fields),
                ..
            }) => selected_fields.fields.merge_in_place(fields),
            Self::DeleteRecord(DeleteRecord {
                selected_fields: None, ..
            }) => (),
            Self::UpdateManyRecords(_) => (),
            Self::DeleteManyRecords(_) => (),
            Self::ConnectRecords(_) => (),
            Self::DisconnectRecords(_) => (),
            Self::ExecuteRaw(_) => (),
            Self::QueryRaw(_) => (),
            Self::Upsert(_) => (),
        }
    }

    pub fn model(&self) -> Model {
        match self {
            Self::CreateRecord(q) => q.model.clone(),
            Self::CreateManyRecords(q) => q.model.clone(),
            Self::UpdateRecord(q) => q.model().clone(),
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
            Self::CreateRecord(q) => write!(
                f,
                "CreateRecord(model: {}, args: {:?}, selected_fields: {:?})",
                q.model.name(),
                q.args,
                q.selected_fields.to_string()
            ),
            Self::CreateManyRecords(q) => write!(f, "CreateManyRecord(model: {})", q.model.name()),
            Self::UpdateRecord(q) => write!(
                f,
                "UpdateRecord(model: {}, filter: {:?}, args: {:?}, selected_fields: {:?})",
                q.model().name(),
                q.record_filter(),
                q.args(),
                q.selected_fields().map(|field| field.to_string()),
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
            Self::CreateManyRecords(q) => format!(
                "CreateManyRecord(model: {}, selected_fields: {:?})",
                q.model.name(),
                q.selected_fields
            ),
            Self::UpdateRecord(q) => format!(
                "UpdateRecord(model: {}, selection: {:?})",
                q.model().name(),
                q.selected_fields()
            ),
            Self::DeleteRecord(q) => format!(
                "DeleteRecord: {}, {:?}, {:?}",
                q.model.name(),
                q.record_filter,
                q.selected_fields
            ),
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
    pub name: String,
    pub model: Model,
    pub args: Vec<WriteArgs>,
    pub skip_duplicates: bool,
    /// Fields of created records that client has requested to return.
    /// `None` if the connector does not support returning the created rows.
    pub selected_fields: Option<CreateManyRecordsFields>,
    /// If set to true, connector will perform the operation using multiple bulk `INSERT` queries.
    /// One query will be issued per a unique set of fields present in the batch. For example, if
    /// `args` contains records:
    ///   {a: 1, b: 1}
    ///   {a: 2, b: 2}
    ///   {a: 3, b: 3, c: 3}
    /// Two queries will be issued: one containing first two records and one for the last record.
    pub split_by_shape: bool,
}

#[derive(Debug, Clone)]
pub struct CreateManyRecordsFields {
    pub fields: FieldSelection,
    pub order: Vec<String>,
    pub nested: Vec<ReadQuery>,
}

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum UpdateRecord {
    /// Update with explicitly selected fields.
    WithSelection(UpdateRecordWithSelection),
    /// Update without any selection set. A subsequent read is required to fulfill other nodes requirements.
    WithoutSelection(UpdateRecordWithoutSelection),
}

impl UpdateRecord {
    pub(crate) fn args(&self) -> &WriteArgs {
        match self {
            UpdateRecord::WithSelection(u) => &u.args,
            UpdateRecord::WithoutSelection(u) => &u.args,
        }
    }

    pub(crate) fn model(&self) -> &Model {
        match self {
            UpdateRecord::WithSelection(u) => &u.model,
            UpdateRecord::WithoutSelection(u) => &u.model,
        }
    }

    pub(crate) fn record_filter(&self) -> &RecordFilter {
        match self {
            UpdateRecord::WithSelection(u) => &u.record_filter,
            UpdateRecord::WithoutSelection(u) => &u.record_filter,
        }
    }

    pub(crate) fn record_filter_mut(&mut self) -> &mut RecordFilter {
        match self {
            UpdateRecord::WithSelection(u) => &mut u.record_filter,
            UpdateRecord::WithoutSelection(u) => &mut u.record_filter,
        }
    }

    pub(crate) fn selected_fields(&self) -> Option<FieldSelection> {
        match self {
            UpdateRecord::WithSelection(u) => Some(u.selected_fields.clone()),
            UpdateRecord::WithoutSelection(_) => None,
        }
    }

    pub(crate) fn set_record_filter(&mut self, record_filter: RecordFilter) {
        match self {
            UpdateRecord::WithSelection(u) => u.record_filter = record_filter,
            UpdateRecord::WithoutSelection(u) => u.record_filter = record_filter,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateRecordWithSelection {
    pub name: String,
    pub model: Model,
    pub record_filter: RecordFilter,
    pub args: WriteArgs,
    pub selected_fields: FieldSelection,
    pub selection_order: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateRecordWithoutSelection {
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
    pub name: String,
    pub model: Model,
    pub record_filter: Option<RecordFilter>,
    /// Fields of the deleted record that client has requested to return.
    /// `None` if the connector does not support returning the deleted row.
    pub selected_fields: Option<DeleteRecordFields>,
}

#[derive(Debug, Clone)]
pub struct DeleteRecordFields {
    pub fields: FieldSelection,
    pub order: Vec<String>,
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
        match self {
            UpdateRecord::WithSelection(u) => Some(&mut u.record_filter.filter),
            UpdateRecord::WithoutSelection(u) => Some(&mut u.record_filter.filter),
        }
    }

    fn set_filter(&mut self, filter: Filter) {
        self.record_filter_mut().filter = filter
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
        self.record_filter_mut().selectors = Some(selectors);
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
