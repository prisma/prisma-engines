//! The `@default` attribute rendering.

use crate::{
    datamodel_calculator::OutputContext,
    pair::{DefaultKind, ScalarFieldPair},
};
use datamodel_renderer::{
    datamodel as renderer,
    value::{Constant, Function, Text, Value},
};

/// Render a default value for the given scalar field.
pub(crate) fn render<'a>(
    field: ScalarFieldPair<'a>,
    output: &mut OutputContext<'a>,
) -> Option<renderer::DefaultValue<'a>> {
    let mut rendered = match field.default().kind() {
        Some(kind) => match kind {
            DefaultKind::Sequence(sequence) => {
                let mut fun = Function::new("sequence");

                if sequence.min_value != 1 {
                    fun.push_param(("minValue", Constant::from(sequence.min_value)));
                }

                if sequence.max_value != i64::MAX {
                    fun.push_param(("maxValue", Constant::from(sequence.max_value)));
                }

                if sequence.cache_size != 1 {
                    fun.push_param(("cache", Constant::from(sequence.cache_size)));
                }

                if sequence.increment_by != 1 {
                    fun.push_param(("increment", Constant::from(sequence.increment_by)));
                }

                if sequence.start_value != 1 {
                    fun.push_param(("start", Constant::from(sequence.start_value)));
                }

                Some(renderer::DefaultValue::function(fun))
            }
            DefaultKind::DbGenerated(default_string) => {
                let mut fun = Function::new("dbgenerated");

                if let Some(param) = default_string.filter(|s| !s.trim_matches('\0').is_empty()) {
                    fun.push_param(Value::from(Text::new(param)));
                }

                Some(renderer::DefaultValue::function(fun))
            }
            DefaultKind::Autoincrement => Some(renderer::DefaultValue::function(Function::new("autoincrement"))),
            DefaultKind::Uuid => Some(renderer::DefaultValue::function(Function::new("uuid"))),
            DefaultKind::Cuid => Some(renderer::DefaultValue::function(Function::new("cuid"))),
            DefaultKind::Now => Some(renderer::DefaultValue::function(Function::new("now"))),
            DefaultKind::String(s) => Some(renderer::DefaultValue::text(s)),
            DefaultKind::Constant(c) => Some(renderer::DefaultValue::constant(c)),
            DefaultKind::EnumVariant(c) => Some(renderer::DefaultValue::constant(c)),
            DefaultKind::Bytes(b) => Some(renderer::DefaultValue::bytes(b)),
            DefaultKind::StringList(vals) => {
                let vals = vals.into_iter().map(Text::new).collect();
                Some(renderer::DefaultValue::array(vals))
            }
            DefaultKind::ConstantList(vals) => Some(renderer::DefaultValue::array(vals)),
            DefaultKind::BytesList(vals) => {
                let vals = vals.into_iter().map(Value::from).collect();
                Some(renderer::DefaultValue::array(vals))
            }
            DefaultKind::Prisma1Uuid => {
                let warn = crate::warnings::ModelAndField {
                    model: field.model().name().to_string(),
                    field: field.name().to_string(),
                };

                output.warnings.prisma_1_uuid_defaults.push(warn);
                Some(renderer::DefaultValue::function(Function::new("uuid")))
            }
            DefaultKind::Prisma1Cuid => {
                let warn = crate::warnings::ModelAndField {
                    model: field.model().name().to_string(),
                    field: field.name().to_string(),
                };

                output.warnings.prisma_1_cuid_defaults.push(warn);
                Some(renderer::DefaultValue::function(Function::new("cuid")))
            }
        },
        None => None,
    };

    if let Some(res) = rendered.as_mut() {
        if let Some(mapped_name) = field.default().mapped_name() {
            res.map(mapped_name);
        }
    }

    rendered
}
