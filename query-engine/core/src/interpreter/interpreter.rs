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
}

impl<'a> QueryInterpreter<'a> {
    pub fn interpret(&mut self, exp: Expression, env: Env) -> InterpretationResult<ExpressionResult> {
        match exp {
            Expression::Func { func } => {
                println!("FUNC");
                self.interpret(func(env.clone())?, env)
            }

            Expression::Sequence { seq } => {
                println!("SEQ");
                seq.into_iter()
                    .map(|exp| self.interpret(exp, env.clone()))
                    .collect::<InterpretationResult<Vec<_>>>()
                    .map(|mut results| results.pop().unwrap_or_else(|| ExpressionResult::Empty))
            }

            Expression::Let { bindings, expressions } => {
                println!("LET");
                let mut inner_env = env.clone();
                for binding in bindings {
                    print!("bind {} ", &binding.name);
                    let result = self.interpret(binding.expr, env.clone())?;
                    inner_env.insert(binding.name, result);
                }

                self.interpret(Expression::Sequence { seq: expressions }, inner_env)
            }

            Expression::Query { query } => match query {
                Query::Read(read) => {
                    println!("READ {} ", read.name());
                    Ok(read::execute(self.tx, read, &[]).map(|res| ExpressionResult::Query(res))?)
                }

                Query::Write(write) => {
                    println!("WRITE");
                    Ok(write::execute(self.tx, write).map(|res| ExpressionResult::Query(res))?)
                }
            },

            Expression::Get { binding_name } => {
                println!("GET {}", binding_name);
                env.clone().remove(&binding_name)
            }

            Expression::GetFirstNonEmpty { binding_names } => Ok(binding_names
                .into_iter()
                .find_map(|binding_name| match env.get(&binding_name) {
                    Some(_) => Some(env.clone().remove(&binding_name).unwrap()),
                    None => None,
                })
                .unwrap()),

            Expression::If { func, then, else_ } => {
                println!("IF");
                if func() {
                    self.interpret(Expression::Sequence { seq: then }, env)
                } else {
                    self.interpret(Expression::Sequence { seq: else_ }, env)
                }
            }
        }
    }
}
