use crate::{CoreResult, QueryInterpreter, QueryGraph, IrSerializer, Expressionista, format_expression, Env, Response};


pub struct QueryPipeline<'a> {
    graph: QueryGraph,
    interpreter: QueryInterpreter<'a>,
    serializer: IrSerializer,
}

impl<'a> QueryPipeline<'a> {
    pub fn new(graph: QueryGraph, interpreter: QueryInterpreter<'a>, serializer: IrSerializer) -> Self {
        Self {
            graph,
            interpreter,
            serializer,
        }
    }

    pub fn execute(mut self) -> CoreResult<Response> {
        let serializer = self.serializer;

        println!("{}", self.graph);
        let expr = Expressionista::translate(self.graph)?;

        println!("{}", format_expression(&expr, 0));
        Ok(self.interpreter
            .interpret(expr, Env::default())
            .map(|result| serializer.serialize(result))?)
    }
}
