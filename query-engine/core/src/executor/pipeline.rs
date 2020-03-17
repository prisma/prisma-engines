use crate::{Env, Expression, Expressionista, IrSerializer, QueryInterpreter, QueryType, Response};

pub struct QueryPipeline<'conn, 'tx> {
    query: QueryType,
    interpreter: QueryInterpreter<'conn, 'tx>,
    serializer: IrSerializer,
}

impl<'conn, 'tx> QueryPipeline<'conn, 'tx> {
    pub fn new(query: QueryType, interpreter: QueryInterpreter<'conn, 'tx>, serializer: IrSerializer) -> Self {
        Self {
            query,
            interpreter,
            serializer,
        }
    }

    pub async fn execute(self) -> crate::Result<Response> {
        let serializer = self.serializer;

        match self.query {
            QueryType::Graph(mut graph) => {
                // Run final validations and transformations.
                graph.finalize()?;
                trace!("{}", graph);

                let expr = Expressionista::translate(graph)?;
                let result = self.interpreter.interpret(expr, Env::default(), 0).await;

                trace!("{}", self.interpreter.log_output());
                Ok(serializer.serialize(result?))
            }
            QueryType::Raw { query, parameters } => {
                trace!("Raw query: {} ({:?})", query, parameters);

                let result = self
                    .interpreter
                    .interpret(Expression::raw(query, parameters), Env::default(), 0)
                    .await;

                trace!("{}", self.interpreter.log_output());

                Ok(serializer.serialize(result?))
            }
        }
    }
}
