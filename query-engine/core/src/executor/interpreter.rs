use super::*;
use crate::WriteQueryResultWrapper;
use connector::Identifier;
use im::HashMap;
use petgraph::visit::EdgeRef;
use prisma_models::prelude::*;
use std::convert::TryInto;
use std::fmt::Debug;

pub enum Expression {
    Sequence {
        seq: Vec<Expression>,
    },
    Func {
        func: Box<dyn FnOnce(Env) -> Expression>,
    },
    Write {
        write: RootWriteQuery,
    },
    Let {
        bindings: Vec<Binding>,
        expressions: Vec<Expression>,
    },
}

impl Debug for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Func { func: _ } => write!(f, "Func\n")?,
            Self::Let { bindings, expressions } => {
                write!(f, "Let {{\n")?;
                write!(f, "\tbindings = [\n")?;
                for b in bindings {
                    write!(f, "\t\t{:?}\n", b)?;
                }
                write!(f, "]\n")?;
                write!(f, "\texpressions = [\n")?;
                for e in expressions {
                    write!(f, "\t\t{:?}\n", e)?;
                }
                write!(f, "]\n")?;
                write!(f, "}}\n")?;
            }
            Self::Sequence { seq } => {
                write!(f, "Sequence {{\n")?;
                write!(f, "\tseq = [\n")?;
                for exp in seq {
                    write!(f, "\t\t{:?}\n", exp)?;
                }
                write!(f, "]\n")?;
                write!(f, "}}\n")?;
            }
            Self::Write { write } => {
                write!(f, "Write {{\n")?;
                write!(f, "\twrite = {:?}\n", write)?;
                write!(f, "}}\n")?;
            }
        };

        Ok(())
    }
}

#[derive(Debug)]
pub struct Binding {
    pub name: String,
    pub exp: Expression,
}

// enum Expression {
//     Let { name: String, exp: Expression }
//     Read { finder: RecordFinder }
//     Write { write: RootWriteQuery }
//     Serialize { key: String, exp: Expression }
//     Get { path: Vec<String>, variable: String }
// }

pub struct Expressionista {}

impl Expressionista {
    pub fn translate(graph: QueryGraph) -> Expression {
        dbg!(&graph);
        let root_nodes: Vec<NodeIndex> = graph
            .node_indices()
            .filter_map(|ix| {
                if let Some(_) = graph.edges_directed(ix, Direction::Incoming).next() {
                    None
                } else {
                    Some(ix)
                }
            })
            .collect();

        let expressions: Vec<Expression> = root_nodes
            .into_iter()
            .map(|ix| {
                let query = graph.node_weight(ix).unwrap();
                let children = graph.edges_directed(ix, Direction::Outgoing).collect::<Vec<_>>();

                match query {
                    Query::Write(WriteQuery::Root(_, _, wq)) => Expression::Let {
                        bindings: vec![Binding {
                            name: "parent".to_owned(),
                            exp: Expression::Write { write: wq.clone() },
                        }],
                        expressions: children
                            .into_iter()
                            .map(|child| {
                                let child_node = graph.node_weight(child.target()).unwrap();
                                let db_name = child.weight().related_field().name.clone();

                                match child_node {
                                    Query::Write(WriteQuery::Root(_, _, wq)) => {
                                        let mut new_writes = wq.clone();
                                        Expression::Func {
                                            func: Box::new(|env: Env| {
                                                let parent_result = env.get("parent").unwrap();
                                                let parent_id = parent_result.as_id();

                                                new_writes.inject_non_list_arg(db_name, parent_id);
                                                dbg!(&new_writes);
                                                Expression::Write { write: new_writes }
                                            }),
                                        }
                                    }
                                    _ => unimplemented!(),
                                }
                            })
                            .collect(),
                    },
                    _ => unimplemented!(), //(Query::Read(_))
                }
            })
            .collect();

        Expression::Sequence { seq: expressions }
    }
}

#[derive(Debug, Clone)]
pub enum ExpressionResult {
    Vec(Vec<ExpressionResult>),
    Write(WriteQueryResultWrapper),
}

impl ExpressionResult {
    pub fn as_id(&self) -> PrismaValue {
        match self {
            Self::Write(wrapper) => match &wrapper.result.identifier {
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

pub struct QueryInterpreter {
    pub writer: WriteQueryExecutor,
}

impl QueryInterpreter {
    pub fn interpret(&self, exp: Expression, env: Env) -> QueryExecutionResult<ExpressionResult> {
        match exp {
            Expression::Func { func } => self.interpret(func(env.clone()), env),
            Expression::Sequence { seq } => seq
                .into_iter()
                .map(|exp| self.interpret(exp, env.clone()))
                .collect::<QueryExecutionResult<Vec<_>>>()
                .map(|res| ExpressionResult::Vec(res)),

            Expression::Let { bindings, expressions } => {
                let mut inner_env = env.clone();
                for binding in bindings {
                    let result = self.interpret(binding.exp, env.clone())?;
                    inner_env.insert(binding.name, result);
                }

                self.interpret(Expression::Sequence { seq: expressions }, inner_env)
            }
            Expression::Write { write } => Ok(self
                .writer
                .execute(WriteQuery::Root("".to_owned(), Some("".to_owned()), write))
                .map(|res| ExpressionResult::Write(res))?),

            _ => unimplemented!(),
        }
    }
}
