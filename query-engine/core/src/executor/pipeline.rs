use crate::{CoreResult, Env, Expressionista, IrSerializer, QueryGraph, QueryInterpreter, Response};

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
        // Run final validations and transformations.
        self.graph.finalize()?;
        trace!("{}", self.graph);

        let serializer = self.serializer;
        let expr = Expressionista::translate(self.graph)?;
        let result = self.interpreter.interpret(expr, Env::default(), 0);

        trace!("{}", self.interpreter.log);
        Ok(serializer.serialize(result?))
    }
}
