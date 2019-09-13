use super::*;
use connector::Identifier;
use connector::{ReadQueryResult, ResultContent};
use im::HashMap;
use prisma_models::prelude::*;
use std::convert::TryInto;

pub enum Expression {
    Sequence {
        seq: Vec<Expression>,
    },

    Func {
        func: Box<dyn FnOnce(Env) -> Expression>,
    },

    Query {
        query: Query,
    },

    Let {
        bindings: Vec<Binding>,
        expressions: Vec<Expression>,
    },

    Get {
        binding_name: String,
    },

    GetFirstNonEmpty {
        binding_names: Vec<String>,
    },

    If {
        func: Box<dyn FnOnce() -> bool>,
        then: Vec<Expression>,
        else_: Vec<Expression>,
    },
}

pub struct Binding {
    pub name: String,
    pub expr: Expression,
}

#[derive(Debug, Clone)]
pub enum ExpressionResult {
    Read(ReadQueryResult),
    Write(WriteQueryResult),
    Empty,
}

impl ExpressionResult {
    /// Wip impl of transforming results into an ID.
    /// todos:
    ///   - Lists are not really handled. Last element wins.
    ///   - Not all result sets are handled.
    pub fn as_id(&self) -> Option<PrismaValue> {
        match self {
            Self::Write(result) => match &result.identifier {
                Identifier::Id(id) => Some(id.clone().try_into().unwrap()),
                _ => unimplemented!(),
            },

            Self::Read(res) => match &res.content {
                ResultContent::RecordSelection(rs) => rs
                    .scalars
                    .collect_ids(rs.id_field.as_str())
                    .unwrap()
                    .pop()
                    .map(|val| val.into()),

                _ => unimplemented!(),
            },
            _ => unimplemented!(),
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

    pub fn remove(&mut self, key: &str) -> QueryExecutionResult<ExpressionResult> {
        match self.env.remove(key) {
            Some(val) => Ok(val),
            None => Err(QueryExecutionError::EnvVarNotFound(key.to_owned())),
        }
    }
}

pub struct QueryInterpreter<'a> {
    pub writer: &'a WriteQueryExecutor,
    pub reader: &'a ReadQueryExecutor,
}

impl<'a> QueryInterpreter<'a> {
    pub fn interpret(&self, exp: Expression, env: Env) -> QueryExecutionResult<ExpressionResult> {
        match exp {
            Expression::Func { func } => {
                println!("FUNC");
                self.interpret(func(env.clone()), env)
            }

            Expression::Sequence { seq } => {
                println!("SEQ");
                seq.into_iter()
                    .map(|exp| self.interpret(exp, env.clone()))
                    .collect::<QueryExecutionResult<Vec<_>>>()
                    .map(|mut results| results.pop().unwrap())
            }

            Expression::Let { bindings, expressions } => {
                println!("LET");
                let mut inner_env = env.clone();
                for binding in bindings {
                    let result = self.interpret(binding.expr, env.clone())?;
                    inner_env.insert(binding.name, result);
                }

                self.interpret(Expression::Sequence { seq: expressions }, inner_env)
            }

            Expression::Query { query } => match query {
                Query::Read(read) => {
                    println!("READ");
                    Ok(self.reader.execute(read, &[]).map(|res| ExpressionResult::Read(res))?)
                }

                Query::Write(write) => {
                    println!("WRITE");
                    Ok(self.writer.execute(write).map(|res| ExpressionResult::Write(res))?)
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
