use crate::{Operation, ResponseData};
use std::fmt::Display;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum TxOpRequestMsg {
    Commit,
    Rollback,
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
            TxOpRequestMsg::Commit => write!(f, "Commit"),
            TxOpRequestMsg::Rollback => write!(f, "Rollback"),
            TxOpRequestMsg::Single(..) => write!(f, "Single"),
            TxOpRequestMsg::Batch(..) => write!(f, "Batch"),
        }
    }
}

#[derive(Debug)]
pub enum TxOpResponse {
    Committed(crate::Result<()>),
    RolledBack(crate::Result<()>),
    Single(crate::Result<ResponseData>),
    Batch(crate::Result<Vec<crate::Result<ResponseData>>>),
}

impl Display for TxOpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Committed(..) => write!(f, "Committed"),
            Self::RolledBack(..) => write!(f, "RolledBack"),
            Self::Single(..) => write!(f, "Single"),
            Self::Batch(..) => write!(f, "Batch"),
        }
    }
}
