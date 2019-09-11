use super::*;
use connector::{Identifier, ReadQuery};
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
    // Write {
    //     write: WriteQuery,
    // },
    // Read {
    //     read: ReadQuery,
    // },
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
}

impl Expression {
    // Todo put into display or whatever.
    pub fn to_string(&self, indent: usize) -> String {
        match self {
            Self::Sequence { seq } => seq
                .into_iter()
                .map(|exp| Self::add_indent(indent, exp.to_string(indent + 2)))
                .collect::<Vec<String>>()
                .join("\n"),

            Self::Query { query } => match query {
                Query::Read(rq) => Self::add_indent(indent, format!("{}", rq)),
                Query::Write(WriteQuery::Root(wq)) => Self::add_indent(indent, format!("{}", wq)),
                _ => unreachable!(),
            },

            Self::Func { func: _ } => Self::add_indent(indent, "(Fn env)"),
            Self::Let { bindings, expressions } => {
                let binding_strs = bindings
                    .into_iter()
                    .map(|binding| {
                        Self::add_indent(
                            indent + 2,
                            format!("(\"{}\" {})", binding.name, binding.exp.to_string(0)),
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                let exp_strs = expressions
                    .into_iter()
                    .map(|exp| exp.to_string(indent))
                    .collect::<Vec<String>>()
                    .join("\n");

                format!("(Let [\n{}\n{}]\n{}\n)", binding_strs, Self::indent(indent), exp_strs)
            }
            Self::Get { binding_name } => Self::add_indent(indent, format!("(Get env '{}')", binding_name)),
        }
    }

    fn add_indent<T: AsRef<str>>(indent: usize, s: T) -> String {
        format!("{}{}", Self::indent(indent), s.as_ref())
    }

    fn indent(indent: usize) -> String {
        " ".repeat(indent)
    }
}

pub struct Binding {
    pub name: String,
    pub exp: Expression,
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
    pub fn as_id(&self) -> PrismaValue {
        match self {
            Self::Write(result) => match &result.identifier {
                Identifier::Id(id) => id.clone().try_into().unwrap(),
                _ => unimplemented!(),
            },
            Self::Read(res) => match &res.content {
                ResultContent::RecordSelection(rs) => rs
                    .scalars
                    .collect_ids(rs.id_field.as_str())
                    .unwrap()
                    .pop()
                    .unwrap()
                    .into(),
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
                    let result = self.interpret(binding.exp, env.clone())?;
                    inner_env.insert(binding.name, result);
                }

                self.interpret(Expression::Sequence { seq: expressions }, inner_env)
            }

            // Expression::Write { write } => {
            //     println!("WRITE");
            //     Ok(self.writer.execute(write).map(|res| ExpressionResult::Write(res))?)
            // }

            // Expression::Read { read } => {
            //     println!("READ");
            //     Ok(self.reader.execute(read, &[]).map(|res| ExpressionResult::Read(res))?)
            // }
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
        }
    }
}
