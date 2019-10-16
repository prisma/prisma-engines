use super::{
    expression::*,
    query_interpreters::{read, write},
    InterpretationResult, InterpreterError,
};
use crate::{Query, QueryResult};
use connector::TransactionLike;
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

pub struct QueryInterpreter<'a> {
    pub(crate) tx: &'a mut dyn TransactionLike,
    log: String,
}

impl<'a> QueryInterpreter<'a> {
    pub fn new(tx: &'a mut dyn TransactionLike) -> QueryInterpreter<'a> {
        QueryInterpreter { tx, log: String::new() }
    }

    pub fn interpret(&mut self, exp: Expression, env: Env, level: usize) -> InterpretationResult<ExpressionResult> {
        match exp {
            Expression::Func { func } => self.interpret(func(env.clone())?, env, level),

            Expression::Sequence { seq } => {
                self.log_line("SEQ".to_string(), level);
                seq.into_iter()
                    .map(|exp| self.interpret(exp, env.clone(), level + 1))
                    .collect::<InterpretationResult<Vec<_>>>()
                    .map(|mut results| results.pop().unwrap_or_else(|| ExpressionResult::Empty))
            }

            Expression::Let {
                bindings,
                mut expressions,
            } => {
                self.log_line("LET".to_string(), level);
                let mut inner_env = env.clone();
                for binding in bindings {
                    self.log_line(format!("bind {} ", &binding.name), level + 1);
                    let result = self.interpret(binding.expr, env.clone(), level + 2)?;
                    inner_env.insert(binding.name, result);
                }
                // the unwrapping improves the readability of the log significantly
                let next_expression = if expressions.len() == 1 {
                    expressions.pop().unwrap()
                } else {
                    Expression::Sequence { seq: expressions }
                };
                self.interpret(next_expression, inner_env, level + 1)
            }

            Expression::Query { query } => match query {
                Query::Read(read) => {
                    self.log_line(format!("READ {}", read), level);
                    Ok(read::execute(self.tx, read, &[]).map(|res| ExpressionResult::Query(res))?)
                }

                Query::Write(write) => {
                    self.log_line(format!("WRITE {}", write), level);
                    Ok(write::execute(self.tx, write).map(|res| ExpressionResult::Query(res))?)
                }
            },

            Expression::Get { binding_name } => {
                self.log_line(format!("GET {}", binding_name), level);
                env.clone().remove(&binding_name)
            }

            Expression::GetFirstNonEmpty { binding_names } => {
                self.log_line(format!("GET FIRST NON EMPTY {:?}", binding_names), level);
                Ok(binding_names
                    .into_iter()
                    .find_map(|binding_name| match env.get(&binding_name) {
                        Some(_) => Some(env.clone().remove(&binding_name).unwrap()),
                        None => None,
                    })
                    .unwrap())
            }

            Expression::If { func, then, else_ } => {
                self.log_line("IF".to_string(), level);
                if func() {
                    self.interpret(Expression::Sequence { seq: then }, env, level + 1)
                } else {
                    self.interpret(Expression::Sequence { seq: else_ }, env, level + 1)
                }
            }
        }
    }

    pub fn print_log(&self) {
        println!("{}", self.log);
    }

    fn log_line(&mut self, s: String, level: usize) {
        let log_line = format!("{:indent$}{}\n", "", s, indent = level * 2);
        self.log.push_str(&log_line);
    }
}
