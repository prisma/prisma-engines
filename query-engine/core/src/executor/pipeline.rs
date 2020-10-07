use crate::{Env, Expressionista, IrSerializer, QueryGraph, QueryInterpreter, ResponseData};

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

    pub async fn execute(self) -> crate::Result<ResponseData> {
        let serializer = self.serializer;
        let mut graph = self.graph;

        // Run final validations and transformations.
        graph.finalize()?;
        trace!("{}", graph);

        let expr = Expressionista::translate(graph)?;
        let result = self.interpreter.interpret(expr, Env::default(), 0).await;

        trace!("{}", self.interpreter.log_output());
        serializer.serialize(result?)
    }
}
