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

impl<'a> GqlTypeRenderer<'a> {
    fn render_input_type(&self, i: &InputType, ctx: &mut RenderContext) -> String {
        match i {
            InputType::Object(ref obj) => {
                let _ = obj.into_renderer().render(ctx);
                obj.into_arc().identifier.name().to_string()
            }

            InputType::Enum(et) => {
                // Not sure how this fits together with the enum handling below.
                let _ = et.into_renderer().render(ctx);
                et.name().to_string()
            }

            InputType::List(ref l) => {
                let substring = self.render_input_type(l, ctx);
                format!("[{}]", substring)
            }

            InputType::Scalar(ScalarType::Enum(et)) => {
                let _ = et.into_renderer().render(ctx);
                et.name().to_string()
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
                    ScalarType::Json => "DateTime",
                    ScalarType::UUID => "UUID",
                    ScalarType::JsonList => "Json",
                    ScalarType::Xml => "Xml",
                    ScalarType::Bytes => "Bytes",
                    ScalarType::Enum(_) => unreachable!("Encountered enum type during GQL scalar rendering."), // Handled separately above.
                    ScalarType::Null => unreachable!("Null types should not be picked for GQL rendering."),
                };

                stringified.to_owned()
            }
        }
    }

    fn render_output_type(&self, o: &OutputType, ctx: &mut RenderContext) -> String {
        match o {
            OutputType::Object(obj) => {
                let _ = obj.into_renderer().render(ctx);
                obj.into_arc().identifier.name().to_string()
            }

            OutputType::Enum(et) => {
                // Not sure how this fits together with the enum handling below.
                let _ = et.into_renderer().render(ctx);
                et.name().to_string()
            }

            OutputType::List(l) => {
                let substring = self.render_output_type(l, ctx);
                format!("[{}]", substring)
            }

            OutputType::Scalar(ScalarType::Enum(et)) => {
                let _ = et.into_renderer().render(ctx);
                et.name().to_string()
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
                    ScalarType::Enum(_) => unreachable!("Encountered enum type during GQL scalar rendering."), // Handled separately above.
                    ScalarType::Null => unreachable!("Null types should not be picked for GQL rendering."),
                };

                format!("{}!", stringified)
            }
        }
    }
}
