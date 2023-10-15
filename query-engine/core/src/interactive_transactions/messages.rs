use crate::{Operation, ResponseData};
use std::fmt::Display;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum TxOpRequestMsg {
    Commit,
    Rollback,
    Begin,
    Single(Operation, Option<String>),
    Batch(Vec<Operation>, Option<String>),
}

pub struct TxOpRequest {
    pub msg: TxOpRequestMsg,
    pub respond_to: oneshot::Sender<TxOpResponse>,
}

impl Display for TxOpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.msg {
            TxOpRequestMsg::Begin => write!(f, "Begin"),
            TxOpRequestMsg::Commit => write!(f, "Commit"),
            TxOpRequestMsg::Rollback => write!(f, "Rollback"),
            TxOpRequestMsg::Single(..) => write!(f, "Single"),
            TxOpRequestMsg::Batch(..) => write!(f, "Batch"),
        }
    }
}

#[derive(Debug)]
pub enum TxOpResponse {
    Begin(crate::Result<()>),
    Committed(crate::Result<i32>),
    RolledBack(crate::Result<i32>),
    Single(crate::Result<ResponseData>),
    Batch(crate::Result<Vec<crate::Result<ResponseData>>>),
}

impl Display for TxOpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Begin(..) => write!(f, "Begin"),
            Self::Committed(..) => write!(f, "Committed"),
            Self::RolledBack(..) => write!(f, "RolledBack"),
            Self::Single(..) => write!(f, "Single"),
            Self::Batch(..) => write!(f, "Batch"),
        }
    }
}
