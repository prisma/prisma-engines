use super::*;
use connector::filter::RecordFinder;
use connector::ReadQuery;

pub struct Expressionista {}

impl Expressionista {
    pub fn translate(graph: QueryGraph) -> Expression {
        let root_nodes: Vec<Node> = graph.root_nodes();
        let expressions = root_nodes
            .into_iter()
            .map(|root_node| Self::build_expression(root_node, None))
            .collect();

        Expression::Sequence { seq: expressions }
    }

    fn build_expression(node: Node, parent_edge: Option<Edge>) -> Expression {
        let query = node.content();
        let exp = Self::query_expression(parent_edge, query);
        let child_edges = node.edges(EdgeDirection::Outgoing);

        // Writes before reads
        let (write_edges, read_edges): (Vec<_>, Vec<_>) =
            child_edges
                .into_iter()
                .partition(|child_edge| match child_edge.content() {
                    EdgeContent::Write(_) => true,
                    EdgeContent::Read(_) => false,
                });

        let mut expressions: Vec<_> = write_edges
            .into_iter()
            .map(|child_edge| Self::build_expression(child_edge.target(), Some(child_edge)))
            .collect();

        let mut read_expressions: Vec<_> = read_edges
            .into_iter()
            .map(|child_edge| Self::build_expression(child_edge.target(), Some(child_edge)))
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

    fn query_expression(parent_edge: Option<Edge>, query: Query) -> Expression {
        match (parent_edge, query) {
            (None, Query::Write(wq)) => Expression::Write { write: wq.clone() },
            (Some(child_edge), Query::Write(wq)) => {
                let mut new_writes = wq.clone();
                let field_name = match child_edge.content() {
                    EdgeContent::Write(rf) => rf.related_field().name.clone(),
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
                    let typ = match child_edge.content() {
                        EdgeContent::Read(ref t) => Arc::clone(t),
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
