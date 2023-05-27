use super::*;

#[derive(Debug)]
pub enum GqlTypeRenderer<'a> {
    Input(&'a InputType),
    Output(&'a OutputType),
}

impl<'a> Renderer for GqlTypeRenderer<'a> {
    fn render(&self, ctx: &mut RenderContext) -> String {
        match self {
            GqlTypeRenderer::Input(i) => self.render_input_type(i, ctx),
            GqlTypeRenderer::Output(o) => self.render_output_type(o, ctx),
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
impl<'a> GqlTypeRenderer<'a> {
    fn render_input_type(&self, i: &InputType, ctx: &mut RenderContext) -> String {
        match i {
            InputType::Object(ref obj) => {
                let _ = obj.as_renderer().render(ctx);
                ctx.query_schema.db[*obj].identifier.name()
            }

            InputType::Enum(et) => {
                let et = &ctx.query_schema.db[*et];
                et.as_renderer().render(ctx);
                et.identifier().name()
            }

            InputType::List(ref l) => {
                let substring = self.render_input_type(l, ctx);
                format!("[{substring}]")
            }

            InputType::Scalar(ref scalar) => {
                let stringified = match scalar {
                    ScalarType::String => "String",
                    ScalarType::Int => "Int",
                    ScalarType::BigInt => "BigInt",
                    ScalarType::Boolean => "Boolean",
                    ScalarType::Float => "Float",
                    ScalarType::Decimal => "Decimal",
                    ScalarType::DateTime => "DateTime",
                    ScalarType::Json => "Json",
                    ScalarType::UUID => "UUID",
                    ScalarType::JsonList => "Json",
                    ScalarType::Xml => "Xml",
                    ScalarType::Bytes => "Bytes",
                    ScalarType::Null => unreachable!("Null types should not be picked for GQL rendering."),
                };

                stringified.to_owned()
            }
        }
    }

    fn render_output_type(&self, o: &OutputType, ctx: &mut RenderContext) -> String {
        match o {
            OutputType::Object(obj) => {
                let _ = obj.as_renderer().render(ctx);
                ctx.query_schema.db[*obj].identifier.name()
            }

            OutputType::Enum(et) => {
                let et = &ctx.query_schema.db[*et];
                et.as_renderer().render(ctx);
                et.identifier().name()
            }

            OutputType::List(l) => {
                let substring = self.render_output_type(l, ctx);
                format!("[{substring}]")
            }

            OutputType::Scalar(ref scalar) => {
                let stringified = match scalar {
                    ScalarType::String => "String",
                    ScalarType::Int => "Int",
                    ScalarType::BigInt => "BigInt",
                    ScalarType::Boolean => "Boolean",
                    ScalarType::Float => "Float",
                    ScalarType::Decimal => "Decimal",
                    ScalarType::DateTime => "DateTime",
                    ScalarType::Json => "Json",
                    ScalarType::UUID => "UUID",
                    ScalarType::JsonList => "Json",
                    ScalarType::Xml => "Xml",
                    ScalarType::Bytes => "Bytes",
                    ScalarType::Null => unreachable!("Null types should not be picked for GQL rendering."),
                };

                stringified.to_string()
            }
        }
    }
}
