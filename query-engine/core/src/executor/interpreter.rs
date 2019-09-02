use super::*;
use connector::{Identifier, ReadQuery};
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
    Write {
        write: WriteQuery,
    },
    Read {
        read: ReadQuery,
    },
    Let {
        bindings: Vec<Binding>,
        expressions: Vec<Expression>,
    },
}

pub struct Binding {
    pub name: String,
    pub exp: Expression,
}

#[derive(Debug, Clone)]
pub enum ExpressionResult {
    Read(ReadQueryResult),
    Write(WriteQueryResult),
}

impl ExpressionResult {
    pub fn as_id(&self) -> PrismaValue {
        match self {
            Self::Write(result) => match &result.identifier {
                Identifier::Id(id) => id.clone().try_into().unwrap(),
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
    pub fn get(&self, key: &str) -> QueryExecutionResult<&ExpressionResult> {
        match self.env.get(key) {
            Some(env) => Ok(env),
            None => Err(QueryExecutionError::InvalidEnv(key.to_owned())),
        }
    }

    pub fn insert(&mut self, key: String, value: ExpressionResult) {
        self.env.insert(key, value);
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
                    let result = self.interpret(binding.exp, env.clone())?;
                    inner_env.insert(binding.name, result);
                }

                self.interpret(Expression::Sequence { seq: expressions }, inner_env)
            }

            Expression::Write { write } => {
                println!("WRITE");
                Ok(self.writer.execute(write).map(|res| ExpressionResult::Write(res))?)
            }

            Expression::Read { read } => {
                println!("READ");
                Ok(self.reader.execute(read, &[]).map(|res| ExpressionResult::Read(res))?)
            }
        }
    }
}
