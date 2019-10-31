use super::{
    expression::*,
    query_interpreters::{read, write},
    InterpretationResult, InterpreterError,
};
use crate::{Query, QueryResult};
use async_std::sync::Mutex;
use connector::Transaction;
use futures::future::{BoxFuture, FutureExt};
use im::HashMap;
use prisma_models::prelude::*;

#[derive(Debug, Clone)]
pub enum ExpressionResult {
    Query(QueryResult),
    Empty,
}

impl ExpressionResult {
    /// Attempts to transform the result into a vector of IDs (as PrismaValue).
    pub fn as_ids(&self) -> Option<Vec<PrismaValue>> {
        match self {
            Self::Query(ref result) => match result {
                QueryResult::Id(id) => Some(vec![id.clone().into()]),

                // We always select IDs, the unwraps are safe.
                QueryResult::RecordSelection(rs) => Some(
                    rs.scalars
                        .collect_ids(rs.id_field.as_str())
                        .unwrap()
                        .into_iter()
                        .map(|val| val.into())
                        .collect(),
                ),

                _ => None,
            },

            _ => None,
        }
    }
}

#[derive(Default, Clone)]
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

pub struct QueryInterpreter<'a, 'b> {
    pub(crate) tx: &'a Box<dyn Transaction<'b> + 'b>,
    pub log: Mutex<String>,
}

impl<'a, 'b> QueryInterpreter<'a, 'b>
where
    'b: 'a,
{
    pub fn new(tx: &'a Box<dyn Transaction<'b> + 'b>) -> QueryInterpreter<'a, 'b> {
        QueryInterpreter {
            tx,
            log: Mutex::new(String::new()),
        }
    }

    pub fn interpret(
        &'a self,
        exp: Expression,
        env: Env,
        level: usize,
    ) -> BoxFuture<'a, InterpretationResult<ExpressionResult>> {
        match exp {
            Expression::Func { func } => async move { self.interpret(func(env.clone())?, env, level).await }.boxed(),

            Expression::Sequence { seq } if seq.is_empty() => async { Ok(ExpressionResult::Empty) }.boxed(),

            Expression::Sequence { seq } => {
                let fut = async move {
                    self.log_line("SEQ".to_string(), level).await;

                    let mut results = vec![];

                    for expr in seq {
                        results.push(self.interpret(expr, env.clone(), level + 1).await?);
                    }

                    Ok(results.pop().unwrap())
                };

                fut.boxed()
            }

            Expression::Let {
                bindings,
                mut expressions,
            } => {
                let fut = async move {
                    let mut inner_env = env.clone();
                    self.log_line("LET".to_string(), level).await;

                    for binding in bindings {
                        let log_line = format!("bind {} ", &binding.name);
                        self.log_line(log_line, level + 1).await;

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
                };

                fut.boxed()
            }

            Expression::Query { query } => {
                let fut = async move {
                    match query {
                        Query::Read(read) => {
                            let log_line = format!("READ {}", read);

                            self.log_line(log_line, level).await;
                            Ok(read::execute(self.tx, read, &[])
                                .await
                                .map(|res| ExpressionResult::Query(res))?)
                        }

                        Query::Write(write) => {
                            let log_line = format!("WRITE {}", write);

                            self.log_line(log_line, level).await;
                            Ok(write::execute(self.tx, write)
                                .await
                                .map(|res| ExpressionResult::Query(res))?)
                        }
                    }
                };
                fut.boxed()
            }

            Expression::Get { binding_name } => {
                let fut = async move {
                    let log_line = format!("GET {}", binding_name);

                    self.log_line(log_line, level).await;
                    env.clone().remove(&binding_name)
                };

                fut.boxed()
            }

            Expression::GetFirstNonEmpty { binding_names } => {
                let fut = async move {
                    let log_line = format!("GET FIRST NON EMPTY {:?}", binding_names);

                    self.log_line(log_line, level).await;
                    Ok(binding_names
                        .into_iter()
                        .find_map(|binding_name| match env.get(&binding_name) {
                            Some(_) => Some(env.clone().remove(&binding_name).unwrap()),
                            None => None,
                        })
                        .unwrap())
                };

                fut.boxed()
            }

            Expression::If {
                func,
                then,
                else_: elze,
            } => {
                let fut = async move {
                    self.log_line("IF".to_string(), level).await;

                    if func() {
                        self.interpret(Expression::Sequence { seq: then }, env, level + 1).await
                    } else {
                        self.interpret(Expression::Sequence { seq: elze }, env, level + 1).await
                    }
                };

                fut.boxed()
            }
        }
    }

    async fn log_line(&self, s: String, level: usize) {
        let log_line = format!("{:indent$}{}\n", "", s, indent = level * 2);
        self.log.lock().await.push_str(&log_line);
    }
}
