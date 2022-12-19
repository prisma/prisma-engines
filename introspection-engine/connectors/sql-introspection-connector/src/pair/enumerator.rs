use crate::sanitize_datamodel_names::ModelName;
use psl::{
    parser_database::walkers,
    schema_ast::ast::{self, WithDocumentation},
};
use sql_schema_describer as sql;
use std::borrow::Cow;

use super::Pair;

pub(crate) type EnumPair<'a> = Pair<'a, walkers::EnumWalker<'a>, sql::EnumWalker<'a>>;
pub(crate) type EnumVariantPair<'a> = Pair<'a, walkers::EnumValueWalker<'a>, sql::EnumVariantWalker<'a>>;

impl<'a> EnumPair<'a> {
    /// The documentation on top of the enum.
    pub(crate) fn documentation(self) -> Option<&'a str> {
        self.previous.and_then(|enm| enm.ast_enum().documentation())
    }

    /// The mapped name, if defined, is the actual name of the enum in
    /// the database.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        match self.context.enum_prisma_name(self.next.id) {
            ModelName::FromPsl { mapped_name, .. } => mapped_name,
            ModelName::RenamedReserved { mapped_name } => Some(mapped_name),
            ModelName::RenamedSanitized { mapped_name } => Some(mapped_name),
            ModelName::FromSql { .. } => None,
        }
    }

    /// Name of the enum in the PSL. The value can be sanitized if it
    /// contains characters that are not allowed in the PSL
    /// definition.
    pub(crate) fn name(self) -> Cow<'a, str> {
        self.context.enum_prisma_name(self.next.id).prisma_name()
    }

    /// The namespace of the enumerator, if using the multi-schema feature.
    pub(crate) fn namespace(self) -> Option<&'a str> {
        if self.context.uses_namespaces() {
            self.next.namespace()
        } else {
            None
        }
    }

    /// The position of the enum from the PSL, if existing. Used for
    /// sorting the enums in the final introspected data model.
    pub(crate) fn previous_position(self) -> Option<ast::EnumId> {
        self.previous.map(|e| e.id)
    }

    /// Iterates all of the variants that are part of the enum.
    pub(crate) fn variants(self) -> impl ExactSizeIterator<Item = EnumVariantPair<'a>> + 'a {
        self.next.variants().map(move |next| {
            let variant_name = self.context.enum_variant_name(next.id);
            let prisma_name = variant_name.prisma_name();

            let previous = self.previous.and_then(|prev| {
                prev.values()
                    .find(|val| val.database_name() == variant_name.mapped_name().unwrap_or(&prisma_name))
            });

            Pair::new(self.context, previous, next)
        })
    }
}

impl<'a> EnumVariantPair<'a> {
    /// The documentation on top of the enum.
    pub(crate) fn documentation(self) -> Option<&'a str> {
        self.previous.and_then(|variant| variant.documentation())
    }

    /// The mapped name, if defined, is the actual name of the variant in
    /// the database.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        self.context.enum_variant_name(self.next.id).mapped_name()
    }

    /// Name of the variant in the PSL. The value can be sanitized if
    /// it contains characters that are not allowed in the PSL
    /// definition.
    pub(crate) fn name(self) -> Cow<'a, str> {
        let name = self.context.enum_variant_name(self.next.id).prisma_name();

        // If the variant is sanitized as an empty string, we will
        // comment the variant out and add a warning.
        //
        // The commented out variant cannot have an empty name, so we
        // just print the non-sanitized one.
        if name.is_empty() {
            Cow::Borrowed(self.next.name())
        } else {
            name
        }
    }
}
