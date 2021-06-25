use crate::{Env, Expressionista, IrSerializer, QueryGraph, QueryInterpreter, ResponseData};

#[derive(Debug)]
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

    pub async fn execute(mut self) -> crate::Result<ResponseData> {
        let serializer = self.serializer;
        let expr = Expressionista::translate(self.graph)?;
        let result = self.interpreter.interpret(expr, Env::default(), 0).await;

        trace!("{}", self.interpreter.log_output());
        serializer.serialize(result?)
    }
}
