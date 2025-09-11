use crate::{Enum, EnumId, EnumVariant, EnumVariantId, Walker};

/// Traverse an enum.
pub type EnumWalker<'a> = Walker<'a, EnumId>;

/// Traverse an enum variant.
pub type EnumVariantWalker<'a> = Walker<'a, EnumVariantId>;

impl<'a> EnumWalker<'a> {
    fn get(self) -> &'a Enum {
        &self.schema.enums[self.id.0 as usize]
    }

    /// The namespace the enum belongs to, if defined.
    pub fn explicit_namespace(self) -> Option<&'a str> {
        self.schema
            .namespaces
            .get_index(self.get().namespace_id.0 as usize)
            .map(|s| s.as_str())
    }

    /// The namespace the enum belongs to, if defined.
    /// If not, falls back to the schema's default namespace, if any.
    pub fn namespace(self) -> Option<&'a str> {
        self.explicit_namespace().or(self.schema.default_namespace.as_deref())
    }

    /// The name of the enum. This is a made up name on MySQL.
    pub fn name(self) -> &'a str {
        &self.get().name
    }

    /// The variants of the enum.
    pub fn variants(self) -> impl ExactSizeIterator<Item = EnumVariantWalker<'a>> {
        super::range_for_key(&self.schema.enum_variants, self.id, |variant| variant.enum_id)
            .map(move |idx| self.walk(EnumVariantId(idx as u32)))
    }

    /// The names of the variants of the enum.
    pub fn values(self) -> impl ExactSizeIterator<Item = &'a str> {
        super::range_for_key(&self.schema.enum_variants, self.id, |variant| variant.enum_id)
            .map(move |idx| self.schema.enum_variants[idx].variant_name.as_str())
    }

    /// Description (comment) of the enum.
    pub fn description(self) -> Option<&'a str> {
        self.get().description.as_deref()
    }
}

impl<'a> EnumVariantWalker<'a> {
    fn get(self) -> &'a EnumVariant {
        &self.schema.enum_variants[self.id.0 as usize]
    }

    /// The parent enum.
    pub fn r#enum(self) -> EnumWalker<'a> {
        self.walk(self.get().enum_id)
    }

    /// The variant itself.
    pub fn name(self) -> &'a str {
        &self.get().variant_name
    }
}
