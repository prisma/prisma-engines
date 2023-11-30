use super::{
    expression::*,
    query_interpreters::{read, write},
    InterpretationResult, InterpreterError,
};
use crate::{Query, QueryResult};
use connector::ConnectionLike;
use futures::future::BoxFuture;
use query_structure::prelude::*;
use std::{collections::HashMap, fmt};
use tracing::Instrument;

#[derive(Debug, Clone)]
pub(crate) enum ExpressionResult {
    /// A result from a query execution.
    Query(QueryResult),

    /// A fixed result returned in the query graph.
    FixedResult(Vec<SelectionResult>),

    /// A result from a computation in the query graph.
    Computation(ComputationResult),

    /// An empty result
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
    pub left: Vec<SelectionResult>,
    pub right: Vec<SelectionResult>,
}

impl DiffResult {
    pub fn is_empty(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }
}

impl ExpressionResult {
    /// Attempts to transform this `ExpressionResult` into a vector of `SelectionResult`s corresponding to the passed desired selection shape.
    /// A vector is returned as some expression results return more than one result row at once.
    pub fn as_selection_results(&self, field_selection: &FieldSelection) -> InterpretationResult<Vec<SelectionResult>> {
        let converted = match self {
            Self::Query(ref result) => match result {
                QueryResult::Id(id) => match id {
                    Some(id) if field_selection.matches(id) => Some(vec![id.project(field_selection)]),
                    None => Some(vec![]),
                    Some(id) => {
                        trace!(
                            "Selection result {:?} does not match field selection {:?}",
                            id,
                            field_selection
                        );
                        None
                    }
                },

                // We always select IDs, the unwraps are safe.
                QueryResult::RecordSelection(Some(rs)) => Some(
                    rs.scalars
                        .extract_selection_results(field_selection)
                        .expect("Expected record selection to contain required model ID fields.")
                        .into_iter()
                        .collect(),
                ),
                QueryResult::RecordSelection(None) => Some(vec![]),

                _ => None,
            },

            Self::FixedResult(p) => p
                .clone()
                .into_iter()
                .map(|sr| field_selection.assimilate(sr))
                .collect::<std::result::Result<Vec<_>, _>>()
                .ok(),

            _ => None,
        };

        converted.ok_or_else(|| {
            InterpreterError::InterpretationError(
                "Unable to convert expression result into a set of selection results".to_owned(),
                None,
            )
        })
    }

    pub fn as_query_result(&self) -> InterpretationResult<&QueryResult> {
        let converted = match self {
            Self::Query(ref q) => Some(q),
            _ => None,
        };

        converted.ok_or_else(|| {
            InterpreterError::InterpretationError("Unable to convert result into a query result".to_owned(), None)
        })
    }

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
pub(crate) struct Env {
    env: HashMap<String, ExpressionResult>,
}

impl Env {
    pub(crate) fn get(&self, key: &str) -> Option<&ExpressionResult> {
        self.env.get(key)
    }

    pub(crate) fn insert(&mut self, key: String, value: ExpressionResult) {
        self.env.insert(key, value);
    }

    pub(crate) fn remove(&mut self, key: &str) -> InterpretationResult<ExpressionResult> {
        match self.env.remove(key) {
            Some(val) => Ok(val),
            None => Err(InterpreterError::EnvVarNotFound(key.to_owned())),
        }
    }
}

pub(crate) struct QueryInterpreter<'conn> {
    pub(crate) conn: &'conn mut dyn ConnectionLike,
    log: Vec<String>,
}

impl<'conn> fmt::Debug for QueryInterpreter<'conn> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryInterpreter").finish()
    }
}

impl<'conn> QueryInterpreter<'conn> {
    fn log_enabled() -> bool {
        tracing::level_filters::STATIC_MAX_LEVEL == tracing::level_filters::LevelFilter::TRACE
    }

    pub(crate) fn new(conn: &'conn mut dyn ConnectionLike) -> QueryInterpreter<'conn> {
        let mut log = Vec::new();

        if Self::log_enabled() {
            log.push("\n".to_string());
        }

        Self { conn, log }
    }

    pub(crate) fn interpret(
        &mut self,
        exp: Expression,
        env: Env,
        level: usize,
        trace_id: Option<String>,
    ) -> BoxFuture<'_, InterpretationResult<ExpressionResult>> {
        match exp {
            Expression::Func { func } => {
                let expr = func(env.clone());

                Box::pin(async move { self.interpret(expr?, env, level, trace_id).await })
            }

            Expression::Sequence { seq } if seq.is_empty() => Box::pin(async { Ok(ExpressionResult::Empty) }),

            Expression::Sequence { seq } => {
                Box::pin(async move {
                    self.log_line(level, || "SEQ");

                    let mut results = Vec::with_capacity(seq.len());

                    for expr in seq {
                        results.push(self.interpret(expr, env.clone(), level + 1, trace_id.clone()).await?);
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

                        let result = self
                            .interpret(binding.expr, env.clone(), level + 2, trace_id.clone())
                            .await?;
                        inner_env.insert(binding.name, result);
                    }

                    // the unwrapping improves the readability of the log significantly
                    let next_expression = if expressions.len() == 1 {
                        expressions.pop().unwrap()
                    } else {
                        Expression::Sequence { seq: expressions }
                    };

                    self.interpret(next_expression, inner_env, level + 1, trace_id).await
                })
            }

            Expression::Query { query } => Box::pin(async move {
                match *query {
                    Query::Read(read) => {
                        self.log_line(level, || format!("READ {read}"));
                        let span = info_span!("prisma:engine:read-execute");
                        Ok(read::execute(self.conn, read, None, trace_id)
                            .instrument(span)
                            .await
                            .map(ExpressionResult::Query)?)
                    }

                    Query::Write(write) => {
                        self.log_line(level, || format!("WRITE {write}"));
                        let span = info_span!("prisma:engine:write-execute");
                        Ok(write::execute(self.conn, write, trace_id)
                            .instrument(span)
                            .await
                            .map(ExpressionResult::Query)?)
                    }
                }
            }),

            Expression::Get { binding_name } => Box::pin(async move {
                self.log_line(level, || format!("GET {binding_name}"));
                env.clone().remove(&binding_name)
            }),

            Expression::GetFirstNonEmpty { binding_names } => Box::pin(async move {
                self.log_line(level, || format!("GET FIRST NON EMPTY {binding_names:?}"));

                Ok(binding_names
                    .into_iter()
                    .find_map(|binding_name| {
                        env.get(&binding_name)
                            .map(|_| env.clone().remove(&binding_name).unwrap())
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
                    self.interpret(Expression::Sequence { seq: then }, env, level + 1, trace_id)
                        .await
                } else {
                    self.interpret(Expression::Sequence { seq: elze }, env, level + 1, trace_id)
                        .await
                }
            }),

            Expression::Return { result } => Box::pin(async move {
                self.log_line(level, || "RETURN");
                Ok(*result)
            }),
        }
    }

    pub(crate) fn log_output(&self) -> String {
        let mut output = String::with_capacity(self.log.len() * 30);

        for s in self.log.iter().rev() {
            output.push_str(s)
        }

        output
    }

    fn log_line<F, S>(&mut self, level: usize, f: F)
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
