//! Write query AST
use connector::filter::{Filter, RecordFinder};
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
    SetRecords(SetRecords),
    ResetData(ResetData),
}

impl WriteQuery {
    pub fn inject_non_list_arg(&mut self, key: String, value: PrismaValue) {
        match self {
            Self::CreateRecord(x) => {
                x.non_list_args.insert(key, value);
            }

            Self::UpdateRecord(x) => {
                x.non_list_args.insert(key, value);
            }

            Self::UpdateManyRecords(x) => {
                x.non_list_args.insert(key, value);
            }

            _ => (),
        };
    }
}

impl std::fmt::Display for WriteQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::CreateRecord(q) => write!(f, "CreateRecord: {}", q.model.name),
            Self::UpdateRecord(q) => write!(
                f,
                "UpdateRecord: {:?}",
                q.where_.as_ref().map(|finder| format!(
                    "{}, {} = {:?}",
                    finder.field.model().name,
                    finder.field.name,
                    finder.value
                ))
            ),
            Self::DeleteRecord(q) => write!(
                f,
                "DeleteRecord: {:?}",
                q.where_.as_ref().map(|finder| format!(
                    "{}, {} = {:?}",
                    finder.field.model().name,
                    finder.field.name,
                    finder.value
                ))
            ),
            Self::UpdateManyRecords(q) => write!(f, "UpdateManyRecords: {}", q.model.name),
            Self::DeleteManyRecords(q) => write!(f, "DeleteManyRecords: {}", q.model.name),
            Self::ConnectRecords(_) => write!(f, "ConnectRecords"),
            Self::DisconnectRecords(_) => write!(f, "DisconnectRecords"),
            Self::SetRecords(_) => write!(f, "SetRecords"),
            Self::ResetData(_) => write!(f, "ResetData"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateRecord {
    pub model: ModelRef,
    pub non_list_args: PrismaArgs,
    pub list_args: Vec<(String, PrismaListValue)>,
}

#[derive(Debug, Clone)]
pub struct UpdateRecord {
    pub model: ModelRef,
    pub where_: Option<RecordFinder>,
    pub non_list_args: PrismaArgs,
    pub list_args: Vec<(String, PrismaListValue)>,
}

#[derive(Debug, Clone)]
pub struct UpdateManyRecords {
    pub model: ModelRef,
    pub filter: Filter,
    pub non_list_args: PrismaArgs,
    pub list_args: Vec<(String, PrismaListValue)>,
}

#[derive(Debug, Clone)]
pub struct DeleteRecord {
    pub model: ModelRef,
    pub where_: Option<RecordFinder>,
}

#[derive(Debug, Clone)]
pub struct DeleteManyRecords {
    pub model: ModelRef,
    pub filter: Filter,
}

#[derive(Debug, Clone)]
pub struct ConnectRecords {
    pub parent: Option<GraphqlId>,
    pub child: Option<GraphqlId>,
    pub relation_field: RelationFieldRef,
}

#[derive(Debug, Clone)]
pub struct DisconnectRecords {}

#[derive(Debug, Clone)]
pub struct SetRecords {
    pub wheres: Vec<RecordFinder>,
}

#[derive(Debug, Clone)]
pub struct ResetData {
    pub internal_data_model: InternalDataModelRef,
}

// SET

// #[derive(Debug, Clone)]
// pub struct NestedSet {
//     pub relation_field: Arc<RelationField>,
//     pub wheres: Vec<RecordFinder>,
// }

// // CONNECT

// #[derive(Debug, Clone)]
// pub struct NestedConnect {
//     pub relation_field: RelationFieldRef,
//     pub where_: RecordFinder,
//     pub top_is_create: bool,
// }

// // DISCONNECT

// #[derive(Debug, Clone)]
// pub struct NestedDisconnect {
//     pub relation_field: Arc<RelationField>,
//     pub where_: Option<RecordFinder>,
// }

// // RESET
