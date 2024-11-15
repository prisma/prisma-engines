use super::IntrospectionPair;
use crate::introspection::sanitize_datamodel_names::{EnumVariantName, ModelName};
use psl::{
    parser_database::{self as db, walkers},
    schema_ast::ast::WithDocumentation,
};
use sql_schema_describer as sql;
use std::borrow::Cow;

/// Pairing the PSL enums (previous) to database enums (next).
pub(crate) type EnumPair<'a> = IntrospectionPair<'a, Option<walkers::EnumWalker<'a>>, sql::EnumWalker<'a>>;

/// Pairing the PSL enum values (previous) to database enums (next).
pub(crate) type EnumVariantPair<'a> =
    IntrospectionPair<'a, Option<walkers::EnumValueWalker<'a>>, sql::EnumVariantWalker<'a>>;

impl<'a> EnumPair<'a> {
    /// The documentation on top of the enum.
    pub(crate) fn documentation(self) -> Option<&'a str> {
        self.previous.and_then(|enm| enm.ast_enum().documentation())
    }

    /// The mapped name, if defined, is the actual name of the enum in
    /// the database.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        self.context.enum_prisma_name(self.next.id).mapped_name()
    }

    /// Name of the enum in the PSL. The value can be sanitized if it
    /// contains characters that are not allowed in the PSL
    /// definition.
    pub(crate) fn name(self) -> Cow<'a, str> {
        self.context.enum_prisma_name(self.next.id).prisma_name()
    }

    /// The name of the variant is taken from the PSL.
    pub(crate) fn name_from_psl(self) -> bool {
        matches!(
            self.context.enum_prisma_name(self.next.id),
            ModelName::FromPsl {
                mapped_name: Some(_),
                ..
            }
        )
    }

    /// The namespace of the enumerator, if using the multi-schema feature.
    pub(crate) fn namespace(self) -> Option<&'a str> {
        self.context.uses_namespaces().then(|| self.next.namespace()).flatten()
    }

    /// The position of the enum from the PSL, if existing. Used for
    /// sorting the enums in the final introspected data model.
    pub(crate) fn previous_position(self) -> Option<db::EnumId> {
        self.previous.map(|e| e.id)
    }

    /// True, if enum uses the same name as another top-level item from
    /// a different namespace.
    pub(crate) fn uses_duplicate_name(self) -> bool {
        self.previous.is_none() && !self.context.name_is_unique(self.next.name())
    }

    /// The COMMENT of the enum.
    pub(crate) fn description(self) -> Option<&'a str> {
        self.next.description()
    }

    /// True if we have a new enum and it has a comment.
    pub(crate) fn adds_a_description(self) -> bool {
        self.previous.is_none() && self.description().is_some()
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

            IntrospectionPair::new(self.context, previous, next)
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

    /// The name of the variant is taken from the PSL.
    pub(crate) fn name_from_psl(self) -> bool {
        matches!(
            self.context.enum_variant_name(self.next.id),
            EnumVariantName::FromPsl {
                mapped_name: Some(_),
                ..
            }
        )
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
