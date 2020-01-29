//! Write query AST
use super::FilteredQuery;
use connector::{filter::Filter, WriteArgs};
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
    pub fn inject_all(&mut self, pairs: Vec<(Field, Vec<PrismaValue>)>) {
        pairs
            .into_iter()
            .for_each(|(field, values)| self.inject_field_arg(field, values));
    }

    // Injects PrismaValues into the write arguments based the passed field.
    // If the underlying representation of the field takes multiple values, a compound field is injected.
    // If values are missing (e.g. empty vec passed), `PrismaValue::Null`(s) are written instead.
    pub fn inject_field_arg(&mut self, field: Field, mut values: Vec<PrismaValue>) {
        let args = match self {
            Self::CreateRecord(ref mut x) => &mut x.args,
            Self::UpdateRecord(x) => &mut x.args,
            Self::UpdateManyRecords(x) => &mut x.args,

            _ => return,
        };

        let key = field.name().to_owned();

        match field {
            Field::Scalar(_) => args.insert(key, values.pop().unwrap_or_else(|| PrismaValue::Null)),
            Field::Relation(rf) => {
                // Equalize the values and backing field lengths.
                if values.len() != rf.data_source_fields().len() {
                    values.truncate(rf.data_source_fields().len());

                    for i in 0..(values.len() - rf.data_source_fields().len()) {
                        values.push(PrismaValue::Null);
                    }
                }

                args.insert_compound(key, values)
            }
        };
    }

    pub fn returns(&self, ident: &ModelIdentifier) -> bool {
        let db_names = ident.db_names().map(|n| n.as_str());

        // x.selected_fields.contains_all_db_names(db_names)

        match self {
            Self::CreateRecord(q) => todo!(),
            Self::UpdateRecord(q) => todo!(),
            Self::DeleteRecord(q) => todo!(),
            Self::UpdateManyRecords(q) => todo!(),
            Self::DeleteManyRecords(q) => todo!(),
            Self::ConnectRecords(q) => false,
            Self::DisconnectRecords(q) => false,
            Self::ResetData(q) => false,
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
    pub args: WriteArgs,
}

#[derive(Debug, Clone)]
pub struct UpdateRecord {
    pub model: ModelRef,
    pub where_: Filter,
    pub args: WriteArgs,
}

#[derive(Debug, Clone)]
pub struct UpdateManyRecords {
    pub model: ModelRef,
    pub filter: Filter,
    pub args: WriteArgs,
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
