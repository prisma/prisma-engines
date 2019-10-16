use crate::{format_expression, CoreResult, Env, Expressionista, IrSerializer, QueryGraph, QueryInterpreter, Response};

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

        let serializer = self.serializer;

        println!("{}", self.graph);
        let expr = Expressionista::translate(self.graph)?;

        //        println!("{}", format_expression(&expr, 0));
        let result = self.interpreter.interpret(expr, Env::default(), 0)?;
        self.interpreter.print_log();
        Ok(serializer.serialize(result))
    }
}
