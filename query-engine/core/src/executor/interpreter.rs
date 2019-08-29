use super::*;
use crate::{OutputTypeRef, WriteQueryResultWrapper};
use connector::filter::RecordFinder;
use connector::Identifier;
use connector::ReadQuery;
use im::HashMap;
use petgraph::visit::EdgeRef;
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

        let expressions = root_nodes
            .into_iter()
            .map(|node_id| Self::build_expression(&graph, node_id, None))
            .collect();
        // let expressions = Self::build_expressions(&graph, root_nodes);

        Expression::Sequence { seq: expressions }
    }

    fn build_expression(
        graph: &QueryGraph,
        node_id: NodeIndex,
        parent_edge: Option<EdgeReference<Dependency>>,
    ) -> Expression {
        let query = graph.node_weight(node_id).unwrap();
        let exp = Self::query_expression(parent_edge, query);
        let child_edges = graph.edges_directed(node_id, Direction::Outgoing).collect::<Vec<_>>();

        // Writes before reads
        let (write_edges, read_edges): (Vec<_>, Vec<_>) =
            child_edges.into_iter().partition(|child| match child.weight() {
                Dependency::Write(_) => true,
                Dependency::Read(_) => false,
            });

        let mut expressions: Vec<_> = write_edges
            .into_iter()
            .map(|child_edge| Self::build_expression(graph, child_edge.target(), Some(child_edge)))
            .collect();

        let mut read_expressions: Vec<_> = read_edges
            .into_iter()
            .map(|child_edge| Self::build_expression(graph, child_edge.target(), Some(child_edge)))
            .collect();

        expressions.append(&mut read_expressions);

        if expressions.is_empty() {
            exp
        } else {
            Expression::Let {
                bindings: vec![Binding {
                    name: "parent".to_owned(),
                    exp,
                }],
                expressions: expressions,
            }
        }
    }

    fn query_expression(edge: Option<EdgeReference<Dependency>>, query: &Query) -> Expression {
        match (edge, query) {
            (None, Query::Write(wq)) => Expression::Write { write: wq.clone() },
            (Some(child_edge), Query::Write(wq)) => {
                let mut new_writes = wq.clone();
                let field_name = match child_edge.weight() {
                    Dependency::Write(rf) => rf.related_field().name.clone(),
                    _ => unreachable!(),
                };

                Expression::Func {
                    func: Box::new(|env: Env| {
                        let parent_result = env.get("parent").unwrap();
                        let parent_id = parent_result.as_id();

                        new_writes.inject_non_list_arg(field_name, parent_id);
                        Expression::Write { write: new_writes }
                    }),
                }
            }
            (None, Query::Read(rq)) => unimplemented!(), //Expression::Read { read: ReadQuery::RecordQuery(new_reads), typ },
            (Some(child_edge), Query::Read(rq)) => match rq {
                ReadQuery::RecordQuery(rq) => {
                    let typ = match child_edge.weight() {
                        Dependency::Read(t) => Arc::clone(t),
                        _ => unreachable!(),
                    };

                    let mut new_reads = rq.clone();
                    Expression::Func {
                        func: Box::new(|env: Env| {
                            let parent_result = env.get("parent").unwrap();
                            let parent_id = parent_result.as_id();

                            let finder = RecordFinder {
                                field: new_reads
                                    .selected_fields
                                    .scalar
                                    .first()
                                    .unwrap()
                                    .field
                                    .model()
                                    .fields()
                                    .id()
                                    .clone(),
                                value: parent_id,
                            };

                            new_reads.record_finder = Some(finder);

                            Expression::Read {
                                read: ReadQuery::RecordQuery(new_reads),
                                typ,
                            }
                        }),
                    }
                }
                _ => unimplemented!(),
            },

            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExpressionResult {
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

impl Into<ResultPair> for ExpressionResult {
    fn into(self) -> ResultPair {
        match self {
            Self::Read(r, typ) => ResultPair::Read(r, typ),
            Self::Write(w) => ResultPair::Write(w),
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

            Expression::Read { read, typ } => {
                println!("READ");
                Ok(self
                    .reader
                    .execute(read, &[])
                    .map(|res| ExpressionResult::Read(res, typ))?)
            }

            _ => unimplemented!(),
        }
    }
}
