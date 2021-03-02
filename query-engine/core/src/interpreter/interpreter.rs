use std::fmt;

use super::{
    expression::*,
    query_interpreters::{read, write},
    InterpretationResult, InterpreterError,
};
use crate::{Query, QueryResult};
use connector::ConnectionLike;
use crossbeam_queue::SegQueue;
use futures::future::BoxFuture;
use im::HashMap;
use prisma_models::prelude::*;

#[derive(Debug, Clone)]
pub enum ExpressionResult {
    Query(QueryResult),
    RawProjections(Vec<RecordProjection>),
    Computation(ComputationResult),
    Empty,
}

#[derive(Debug, Clone)]
pub enum ComputationResult {
    Diff(DiffResult),
}

/// Diff of two identifier vectors A and B:
/// `left` contains all elements that are in A but not in B.
/// `right` contains all elements that are in B but not in A.
#[derive(Debug, Clone)]
pub struct DiffResult {
    pub left: Vec<RecordProjection>,
    pub right: Vec<RecordProjection>,
}

impl ExpressionResult {
    /// Attempts to transform the result into a vector of record projections.
    #[tracing::instrument(skip(self, model_projection))]
    pub fn as_projections(&self, model_projection: &ModelProjection) -> InterpretationResult<Vec<RecordProjection>> {
        let converted = match self {
            Self::Query(ref result) => match result {
                QueryResult::Id(id) => match id {
                    Some(id) if model_projection.matches(id) => Some(vec![id.clone()]),
                    None => Some(vec![]),
                    Some(id) => {
                        trace!("RID {:?} does not match MID {:?}", id, model_projection);
                        None
                    }
                },

                // We always select IDs, the unwraps are safe.
                QueryResult::RecordSelection(rs) => Some(
                    rs.scalars
                        .projections(model_projection)
                        .expect("Expected record selection to contain required model ID fields.")
                        .into_iter()
                        .collect(),
                ),

                _ => None,
            },

            Self::RawProjections(p) => p
                .clone()
                .into_iter()
                .map(|p| model_projection.assimilate(p))
                .collect::<std::result::Result<Vec<_>, _>>()
                .ok(),

            _ => None,
        };

        converted.ok_or_else(|| {
            InterpreterError::InterpretationError("Unable to convert result into a set of projections".to_owned(), None)
        })
    }

    #[tracing::instrument(skip(self))]
    pub fn as_query_result(&self) -> InterpretationResult<&QueryResult> {
        let converted = match self {
            Self::Query(ref q) => Some(q),
            _ => None,
        };

        converted.ok_or_else(|| {
            InterpreterError::InterpretationError("Unable to convert result into a query result".to_owned(), None)
        })
    }

    #[tracing::instrument(skip(self))]
    pub fn as_diff_result(&self) -> InterpretationResult<&DiffResult> {
        let converted = match self {
            Self::Computation(ComputationResult::Diff(ref d)) => Some(d),
            _ => None,
        };

        converted.ok_or_else(|| {
            InterpreterError::InterpretationError("Unable to convert result into a computation result".to_owned(), None)
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct Env {
    env: HashMap<String, ExpressionResult>,
}

impl Env {
    pub fn get(&self, key: &str) -> Option<&ExpressionResult> {
        self.env.get(key)
    }

    pub fn insert(&mut self, key: String, value: ExpressionResult) {
        self.env.insert(key, value);
    }

    pub fn remove(&mut self, key: &str) -> InterpretationResult<ExpressionResult> {
        match self.env.remove(key) {
            Some(val) => Ok(val),
            None => Err(InterpreterError::EnvVarNotFound(key.to_owned())),
        }
    }
}

pub struct QueryInterpreter<'conn, 'tx> {
    pub(crate) conn: ConnectionLike<'conn, 'tx>,
    log: SegQueue<String>,
}

impl<'conn, 'tx> fmt::Debug for QueryInterpreter<'conn, 'tx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryInterpreter").finish()
    }
}

impl<'conn, 'tx> QueryInterpreter<'conn, 'tx>
where
    'tx: 'conn,
{
    fn log_enabled() -> bool {
        tracing::level_filters::STATIC_MAX_LEVEL == tracing::level_filters::LevelFilter::TRACE
    }

    pub fn new(conn: ConnectionLike<'conn, 'tx>) -> QueryInterpreter<'conn, 'tx> {
        let log = SegQueue::new();

        if Self::log_enabled() {
            log.push("\n".to_string());
        }

        Self { conn, log }
    }

    #[tracing::instrument(skip(self, exp, env, level))]
    pub fn interpret(
        &'conn self,
        exp: Expression,
        env: Env,
        level: usize,
    ) -> BoxFuture<'conn, InterpretationResult<ExpressionResult>> {
        match exp {
            Expression::Func { func } => {
                let expr = func(env.clone());

                Box::pin(async move { self.interpret(expr?, env, level).await })
            }

            Expression::Sequence { seq } if seq.is_empty() => Box::pin(async { Ok(ExpressionResult::Empty) }),

            Expression::Sequence { seq } => {
                Box::pin(async move {
                    self.log_line(level, || "SEQ");

                    let mut results = Vec::with_capacity(seq.len());

                    for expr in seq {
                        results.push(self.interpret(expr, env.clone(), level + 1).await?);
                    }

                    // Last result gets returned
                    Ok(results.pop().unwrap())
                })
            }

            Expression::Let {
                bindings,
                mut expressions,
            } => {
                Box::pin(async move {
                    let mut inner_env = env.clone();
                    self.log_line(level, || "LET");

                    for binding in bindings {
                        self.log_line(level + 1, || format!("bind {} ", &binding.name));

                        let result = self.interpret(binding.expr, env.clone(), level + 2).await?;
                        inner_env.insert(binding.name, result);
                    }

                    // the unwrapping improves the readability of the log significantly
                    let next_expression = if expressions.len() == 1 {
                        expressions.pop().unwrap()
                    } else {
                        Expression::Sequence { seq: expressions }
                    };

                    self.interpret(next_expression, inner_env, level + 1).await
                })
            }

            Expression::Query { query } => Box::pin(async move {
                match *query {
                    Query::Read(read) => {
                        self.log_line(level, || format!("READ {}", read));
                        Ok(read::execute(&self.conn, read, None)
                            .await
                            .map(ExpressionResult::Query)?)
                    }

                    Query::Write(write) => {
                        self.log_line(level, || format!("WRITE {}", write));
                        Ok(write::execute(&self.conn, write).await.map(ExpressionResult::Query)?)
                    }
                }
            }),

            Expression::Get { binding_name } => Box::pin(async move {
                self.log_line(level, || format!("GET {}", binding_name));
                env.clone().remove(&binding_name)
            }),

            Expression::GetFirstNonEmpty { binding_names } => Box::pin(async move {
                self.log_line(level, || format!("GET FIRST NON EMPTY {:?}", binding_names));

                Ok(binding_names
                    .into_iter()
                    .find_map(|binding_name| match env.get(&binding_name) {
                        Some(_) => Some(env.clone().remove(&binding_name).unwrap()),
                        None => None,
                    })
                    .unwrap())
            }),

            Expression::If {
                func,
                then,
                else_: elze,
            } => Box::pin(async move {
                self.log_line(level, || "IF");

                if func() {
                    self.interpret(Expression::Sequence { seq: then }, env, level + 1).await
                } else {
                    self.interpret(Expression::Sequence { seq: elze }, env, level + 1).await
                }
            }),

            Expression::Return { result } => Box::pin(async move {
                self.log_line(level, || "RETURN");
                Ok(*result)
            }),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn log_output(&self) -> String {
        let mut output = String::with_capacity(self.log.len() * 30);

        while let Ok(s) = self.log.pop() {
            output.push_str(&s);
        }

        output
    }

    fn log_line<F, S>(&self, level: usize, f: F)
    where
        S: AsRef<str>,
        F: FnOnce() -> S,
    {
        if Self::log_enabled() {
            self.log
                .push(format!("{:indent$}{}\n", "", f().as_ref(), indent = level * 2));
        }
    }
}
