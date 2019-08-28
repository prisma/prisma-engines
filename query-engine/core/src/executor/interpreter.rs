use super::*;
use crate::{OutputTypeRef, WriteQueryResultWrapper};
use connector::Identifier;
use connector::ReadQuery;
use im::HashMap;
use petgraph::visit::EdgeRef;
use prisma_models::prelude::*;
use std::convert::TryInto;
use connector::filter::RecordFinder;

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
    Read {
        read: ReadQuery,
        typ: OutputTypeRef,
    },
    Let {
        bindings: Vec<Binding>,
        expressions: Vec<Expression>,
    },
    // Serialize {
    //     key: String,
    //     expression: Box<Expression>,
    // }
}

// impl Debug for Expression {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Func { func: _ } => write!(f, "Func\n")?,
//             Self::Let { bindings, expressions } => {
//                 write!(f, "Let {{\n")?;
//                 write!(f, "\tbindings = [\n")?;
//                 for b in bindings {
//                     write!(f, "\t\t{:?}\n", b)?;
//                 }
//                 write!(f, "]\n")?;
//                 write!(f, "\texpressions = [\n")?;
//                 for e in expressions {
//                     write!(f, "\t\t{:?}\n", e)?;
//                 }
//                 write!(f, "]\n")?;
//                 write!(f, "}}\n")?;
//             }
//             Self::Sequence { seq } => {
//                 write!(f, "Sequence {{\n")?;
//                 write!(f, "\tseq = [\n")?;
//                 for exp in seq {
//                     write!(f, "\t\t{:?}\n", exp)?;
//                 }
//                 write!(f, "]\n")?;
//                 write!(f, "}}\n")?;
//             }
//             Self::Write { write } => {
//                 write!(f, "Write {{\n")?;
//                 write!(f, "\twrite = {:?}\n", write)?;
//                 write!(f, "}}\n")?;
//             },
//             _ => unimplemented!()
//         };

//         Ok(())
//     }
// }

pub struct Binding {
    pub name: String,
    pub exp: Expression,
}

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

        // todo likely recursion missing
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
                        expressions: {
                            dbg!(&children);
                            let (writes, reads): (Vec<_>, Vec<_>) =
                                children.into_iter().partition(|child| match child.weight() {
                                    Dependency::Write(_) => true,
                                    Dependency::Read(_) => false,
                                });

                                dbg!(&writes);
                                dbg!(&reads);

                            let mut writes: Vec<_> = writes
                                .into_iter()
                                .map(|child| {
                                    let child_node = graph.node_weight(child.target()).unwrap();

                                    let db_name = match child.weight() {
                                        Dependency::Write(rf) => rf.related_field().name.clone(),
                                        _ => unreachable!(),
                                    };

                                    match child_node {
                                        Query::Write(WriteQuery::Root(_, _, wq)) => {
                                            let mut new_writes = wq.clone();
                                            Expression::Func {
                                                func: Box::new(|env: Env| {
                                                    let parent_result = env.get("parent").unwrap();
                                                    let parent_id = parent_result.as_id();

                                                    new_writes.inject_non_list_arg(db_name, parent_id);
                                                    Expression::Write { write: new_writes }
                                                }),
                                            }
                                        }
                                        _ => unimplemented!(),
                                    }
                                })
                                .collect();

                            let mut reads = reads
                                .into_iter()
                                .map(|read| {
                                    let read_node = graph.node_weight(read.target()).unwrap();
                                    let typ = match read.weight() {
                                        Dependency::Read(t) => Arc::clone(t),
                                        _ => unreachable!(),
                                    };

                                    match read_node {
                                        Query::Read(rq) => match rq {
                                            ReadQuery::RecordQuery(rq) => {
                                                let mut new_reads = rq.clone();
                                            Expression::Func {
                                                func: Box::new(|env: Env| {
                                                    let parent_result = env.get("parent").unwrap();
                                                    let parent_id = parent_result.as_id();

                                                    let finder = RecordFinder {
                                                        field: new_reads.selected_fields.scalar.first().unwrap().field.model().fields().id().clone(),
                                                        value: parent_id,
                                                    };

                                                    new_reads.record_finder = Some(finder);

                                                    Expression::Read { read: ReadQuery::RecordQuery(new_reads), typ }
                                                }),
                                            }},
                                            _ => unreachable!(),
                                        }
                                        _ => unimplemented!(),
                                    }
                                })
                                .collect();

                                writes.append(&mut reads);
                                writes
                        },
                    },
                    Query::Read(_rq) => unimplemented!(),
                    _ => unimplemented!(),
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
    Read(ReadQueryResult, OutputTypeRef),
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
    pub reader: ReadQueryExecutor,
}

impl QueryInterpreter {
    pub fn interpret(&self, exp: Expression, env: Env) -> QueryExecutionResult<ExpressionResult> {
        match exp {
            Expression::Func { func } => {
                println!("FUNC");
                self.interpret(func(env.clone()), env)
                },
            Expression::Sequence { seq } => {
                println!("SEQ");
                seq
                .into_iter()
                .map(|exp| self.interpret(exp, env.clone()))
                .collect::<QueryExecutionResult<Vec<_>>>()
                .map(|res| ExpressionResult::Vec(res))},

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
                Ok(self
                .writer
                .execute(WriteQuery::Root("".to_owned(), Some("".to_owned()), write))
                .map(|res| ExpressionResult::Write(res))?)},

            Expression::Read { read, typ } => {
                println!("READ");
                Ok(self
                .reader
                .execute(read, &[])
                .map(|res| ExpressionResult::Read(res, typ))?)},

            _ => unimplemented!(),
        }
    }
}
