use crate::{CoreResult, Env, Expressionista, IrSerializer, QueryGraph, QueryInterpreter, Response};

pub struct QueryPipeline<'conn, 'tx> {
    graph: QueryGraph,
    interpreter: QueryInterpreter<'conn, 'tx>,
    serializer: IrSerializer,
}

impl<'conn, 'tx> QueryPipeline<'conn, 'tx> {
    pub fn new(graph: QueryGraph, interpreter: QueryInterpreter<'conn, 'tx>, serializer: IrSerializer) -> Self {
        Self {
            graph,
            interpreter,
            serializer,
        }
    }

    pub async fn execute(mut self) -> CoreResult<Response> {
        // Run final validations and transformations.
        println!("BEFORE: {}", self.graph);
        self.graph.finalize()?;
        trace!("{}", self.graph);
        eprintln!("TEST TEST TEST0");
        println!("AFTER: {}", self.graph);

        let serializer = self.serializer;
        let expr = Expressionista::translate(self.graph)?;
        let result = self.interpreter.interpret(expr, Env::default(), 0).await;

        trace!("{}", self.interpreter.log_output());
        Ok(serializer.serialize(result?))
    }
}
