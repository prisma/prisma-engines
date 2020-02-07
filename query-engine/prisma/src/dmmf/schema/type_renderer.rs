use super::*;

#[derive(Debug)]
pub enum DMMFTypeRenderer<'a> {
    Input(&'a InputType),
    Output(&'a OutputType),
}

impl<'a> Renderer<'a, DMMFTypeInfo> for DMMFTypeRenderer<'a> {
    fn render(&self, ctx: &RenderContext) -> DMMFTypeInfo {
        match self {
            DMMFTypeRenderer::Input(i) => self.render_input_type(i, ctx),
            DMMFTypeRenderer::Output(o) => self.render_output_type(o, ctx),
        }
    }
}

impl<'a> DMMFTypeRenderer<'a> {
    fn render_input_type(&self, i: &InputType, ctx: &RenderContext) -> DMMFTypeInfo {
        match i {
            InputType::Object(ref obj) => {
                obj.into_renderer().render(ctx);
                let type_info = DMMFTypeInfo {
                    typ: obj.into_arc().name.clone(),
                    kind: TypeKind::Object,
                    is_required: true,
                    is_list: false,
                };

                type_info
            }
            InputType::Enum(et) => {
                et.into_renderer().render(ctx);
                let type_info = DMMFTypeInfo {
                    typ: et.name().to_owned(),
                    kind: TypeKind::Enum,
                    is_required: true,
                    is_list: false,
                };

                type_info
            }
            InputType::List(ref l) => {
                let mut type_info = self.render_input_type(l, ctx);
                type_info.is_list = true;

                type_info
            }
            InputType::Opt(ref opt) => {
                let mut type_info = self.render_input_type(opt, ctx);
                type_info.is_required = false;

                type_info
            }
            InputType::Scalar(ScalarType::Enum(et)) => {
                et.into_renderer().render(ctx);
                let type_info = DMMFTypeInfo {
                    typ: et.name().to_owned(),
                    kind: TypeKind::Scalar,
                    is_required: true,
                    is_list: false,
                };

                type_info
            }
            InputType::Scalar(ref scalar) => {
                let stringified = match scalar {
                    ScalarType::String => "String",
                    ScalarType::Int => "Int",
                    ScalarType::Boolean => "Boolean",
                    ScalarType::Float => "Float",
                    ScalarType::DateTime => "DateTime",
                    ScalarType::Json => "Json",
                    ScalarType::UUID => "UUID",
                    ScalarType::JsonList => "Json",
                    ScalarType::Enum(_) => unreachable!(), // Handled separately above.
                };

                let type_info = DMMFTypeInfo {
                    typ: stringified.into(),
                    kind: TypeKind::Scalar,
                    is_required: true,
                    is_list: false,
                };

                type_info
            }
        }
    }

    // WIP dedup code
    fn render_output_type(&self, o: &OutputType, ctx: &RenderContext) -> DMMFTypeInfo {
        match o {
            OutputType::Object(ref obj) => {
                obj.into_renderer().render(ctx);
                let type_info = DMMFTypeInfo {
                    typ: obj.into_arc().name().to_string(),
                    kind: TypeKind::Object,
                    is_required: true,
                    is_list: false,
                };

                type_info
            }
            OutputType::Enum(et) => {
                et.into_renderer().render(ctx);
                let type_info = DMMFTypeInfo {
                    typ: et.name().to_owned(),
                    kind: TypeKind::Enum,
                    is_required: true,
                    is_list: false,
                };

                type_info
            }
            OutputType::List(ref l) => {
                let mut type_info = self.render_output_type(l, ctx);
                type_info.is_list = true;

                type_info
            }
            OutputType::Opt(ref opt) => {
                let mut type_info = self.render_output_type(opt, ctx);
                type_info.is_required = false;

                type_info
            }
            OutputType::Scalar(ScalarType::Enum(et)) => {
                et.into_renderer().render(ctx);
                let type_info = DMMFTypeInfo {
                    typ: et.name().to_owned(),
                    kind: TypeKind::Scalar,
                    is_required: true,
                    is_list: false,
                };

                type_info
            }
            OutputType::Scalar(ref scalar) => {
                let stringified = match scalar {
                    ScalarType::String => "String",
                    ScalarType::Int => "Int",
                    ScalarType::Boolean => "Boolean",
                    ScalarType::Float => "Float",
                    ScalarType::DateTime => "DateTime",
                    ScalarType::Json => "Json",
                    ScalarType::UUID => "UUID",
                    ScalarType::JsonList => "Json",
                    ScalarType::Enum(_) => unreachable!(), // Handled separately above.
                };

                let type_info = DMMFTypeInfo {
                    typ: stringified.into(),
                    kind: TypeKind::Scalar,
                    is_required: true,
                    is_list: false,
                };

                type_info
            }
        }
    }
}
