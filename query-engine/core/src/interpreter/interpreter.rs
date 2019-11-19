use super::{
    expression::*,
    query_interpreters::{read, write},
    InterpretationResult, InterpreterError,
};
use crate::{Query, QueryResult};
use connector::ConnectionLike;
use crossbeam_queue::SegQueue;
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

pub struct QueryInterpreter<'conn, 'tx> {
    pub(crate) conn: ConnectionLike<'conn, 'tx>,
    log: SegQueue<String>,
}

impl<'conn, 'tx> QueryInterpreter<'conn, 'tx>
where
    'tx: 'conn,
{
    pub fn new(conn: ConnectionLike<'conn, 'tx>) -> QueryInterpreter<'conn, 'tx> {
        let log = SegQueue::new();
        log.push("\n".to_string());

        Self { conn, log }
    }

    pub fn interpret(
        &'conn self,
        exp: Expression,
        env: Env,
        level: usize,
    ) -> BoxFuture<'conn, InterpretationResult<ExpressionResult>> {
        match exp {
            Expression::Func { func } => {
                let expr = func(env.clone());

                async move { self.interpret(expr?, env, level).await }.boxed()
            }

            Expression::Sequence { seq } if seq.is_empty() => async { Ok(ExpressionResult::Empty) }.boxed(),

            Expression::Sequence { seq } => {
                let fut = async move {
                    self.log_line("SEQ", level);

                    let mut results = Vec::with_capacity(seq.len());

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
                    self.log_line("LET", level);

                    for binding in bindings {
                        let log_line = format!("bind {} ", &binding.name);
                        self.log_line(log_line, level + 1);

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
                            self.log_line(format!("READ {}", read), level);

                            Ok(read::execute(&self.conn, read, &[])
                                .await
                                .map(|res| ExpressionResult::Query(res))?)
                        }

                        Query::Write(write) => {
                            self.log_line(format!("WRITE {}", write), level);
                            Ok(write::execute(&self.conn, write)
                                .await
                                .map(|res| ExpressionResult::Query(res))?)
                        }
                    }
                };
                fut.boxed()
            }

            Expression::Get { binding_name } => async move {
                self.log_line(format!("GET {}", binding_name), level);
                env.clone().remove(&binding_name)
            }
                .boxed(),

            Expression::GetFirstNonEmpty { binding_names } => {
                let fut = async move {
                    self.log_line(format!("GET FIRST NON EMPTY {:?}", binding_names), level);

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
                    self.log_line("IF", level);

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

    pub fn log_output(&self) -> String {
        let mut output = String::with_capacity(self.log.len() * 30);

        while let Ok(s) = self.log.pop() {
            output.push_str(&s);
        }

        output
    }

    fn log_line<S: AsRef<str>>(&self, s: S, level: usize) {
        self.log
            .push(format!("{:indent$}{}\n", "", s.as_ref(), indent = level * 2));
    }
}
