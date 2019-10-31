use crate::{CoreResult, Env, Expressionista, IrSerializer, QueryGraph, QueryInterpreter, Response};

pub struct QueryPipeline<'a, 'b> {
    graph: QueryGraph,
    interpreter: QueryInterpreter<'a, 'b>,
    serializer: IrSerializer,
}

impl<'a, 'b> QueryPipeline<'a, 'b> {
    pub fn new(graph: QueryGraph, interpreter: QueryInterpreter<'a, 'b>, serializer: IrSerializer) -> Self {
        Self {
            graph,
            interpreter,
            serializer,
        }
    }

    pub async fn execute(mut self) -> CoreResult<Response> {
        // Run final validations and transformations.
        self.graph.finalize()?;
        trace!("{}", self.graph);

        let serializer = self.serializer;
        let expr = Expressionista::translate(self.graph)?;
        let result = self.interpreter.interpret(expr, Env::default(), 0);
        let result = result.await;

        trace!("{}", self.interpreter.log.lock().await);
        Ok(serializer.serialize(result?))
    }
}
