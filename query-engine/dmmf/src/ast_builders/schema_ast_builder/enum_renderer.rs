use super::*;

pub(crate) fn render_enum_types(ctx: &mut RenderContext, enum_types: &[EnumTypeRef]) {
    let mut borrows: Vec<_> = enum_types.iter().collect();

    borrows.sort_by(|a, b| a.name().cmp(b.name()));
    borrows.into_iter().for_each(|et| DmmfEnumRenderer::new(et).render(ctx));
}

pub struct DmmfEnumRenderer {
    enum_type: EnumType,
}

impl Renderer for DmmfEnumRenderer {
    fn render(&self, ctx: &mut RenderContext) {
        let ident = self.enum_type.identifier();
        if ctx.already_rendered(ident) {
            return;
        }

        let values = self.format_enum_values();
        let rendered = DmmfEnum {
            name: self.enum_type.name().to_owned(),
            values,
        };

        ctx.add_enum(ident.clone(), rendered);
    }
}

impl DmmfEnumRenderer {
    pub fn new(enum_type: &EnumType) -> DmmfEnumRenderer {
        DmmfEnumRenderer {
            enum_type: enum_type.clone(),
        }
    }

    fn format_enum_values(&self) -> Vec<String> {
        match &self.enum_type {
            EnumType::String(s) => s.values().to_owned(),
            EnumType::Database(dbt) => dbt.external_values(),
            EnumType::FieldRef(f) => f.values(),
        }
    }
}
