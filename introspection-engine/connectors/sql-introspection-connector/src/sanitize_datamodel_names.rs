//! Some SQL identifiers are not presentable in PSL. The sanitization
//! of these strings happens in this module.

use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

/// Regex to determine if an identifier starts with a character that
/// is not supported.
static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new("^[^a-zA-Z]+").unwrap());

/// Regex to determine if an identifier contains a character that is not
/// supported.
static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

/// If a string has to be sanitized to the PSL.
pub(crate) fn needs_sanitation(s: &str) -> bool {
    RE_START.is_match(s) || RE.is_match(s)
}

/// Sanitize the string to be used in the PSL.
pub(crate) fn sanitize_string(s: &str) -> Cow<'_, str> {
    if needs_sanitation(s) {
        let start_cleaned = RE_START.replace_all(s, "");
        Cow::Owned(RE.replace_all(start_cleaned.as_ref(), "_").into_owned())
    } else {
        Cow::Borrowed(s)
    }
}

/// Names that correspond to _types_ in the generated client.
/// Concretely, enums, models and composite types.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ModelName<'a> {
    FromPsl {
        name: &'a str,
        mapped_name: Option<&'a str>,
    },
    FromSql {
        name: &'a str,
    },
    RenamedReserved {
        mapped_name: &'a str,
    },
    RenamedSanitized {
        mapped_name: &'a str,
    },
}

impl<'a> ModelName<'a> {
    /// Create a name from an SQL identifier.
    pub(crate) fn new_from_sql(name: &'a str) -> Self {
        match name {
            _ if psl::is_reserved_type_name(name) => ModelName::RenamedReserved { mapped_name: name },
            _ if crate::sanitize_datamodel_names::needs_sanitation(name) => {
                ModelName::RenamedSanitized { mapped_name: name }
            }
            name => ModelName::FromSql { name },
        }
    }

    /// Output name to the PSL.
    pub(crate) fn prisma_name(&self) -> Cow<'a, str> {
        match self {
            ModelName::FromPsl { name, .. } => Cow::Borrowed(name),
            ModelName::FromSql { name } => Cow::Borrowed(name),
            ModelName::RenamedReserved { mapped_name } => Cow::Owned(format!("Renamed{mapped_name}")),
            ModelName::RenamedSanitized { mapped_name } => {
                crate::sanitize_datamodel_names::sanitize_string(mapped_name)
            }
        }
    }

    /// The original name to be used in the `@@map` attribute.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        match self {
            ModelName::FromPsl { mapped_name, .. } => mapped_name,
            ModelName::FromSql { .. } => None,
            ModelName::RenamedReserved { mapped_name } => Some(mapped_name),
            ModelName::RenamedSanitized { mapped_name } => Some(mapped_name),
        }
    }
}

/// Names that correspond to _identifiers_ in the generated client.
/// Concretely, columns.
#[derive(Clone, Copy, Debug)]
pub(crate) enum IntrospectedName<'a> {
    FromPsl {
        name: &'a str,
        mapped_name: Option<&'a str>,
    },
    FromSql {
        name: &'a str,
    },
    RenamedSanitized {
        mapped_name: &'a str,
    },
}

impl<'a> IntrospectedName<'a> {
    /// Create a name from an SQL identifier.
    pub(crate) fn new_from_sql(name: &'a str) -> Self {
        match name {
            _ if crate::sanitize_datamodel_names::needs_sanitation(name) => {
                IntrospectedName::RenamedSanitized { mapped_name: name }
            }
            name => IntrospectedName::FromSql { name },
        }
    }

    /// Output name to the PSL.
    pub(crate) fn prisma_name(&self) -> Cow<'a, str> {
        match self {
            IntrospectedName::FromPsl { name, .. } => Cow::Borrowed(name),
            IntrospectedName::FromSql { name } => Cow::Borrowed(name),
            IntrospectedName::RenamedSanitized { mapped_name } => {
                crate::sanitize_datamodel_names::sanitize_string(mapped_name)
            }
        }
    }

    /// The original name to be used in the `@map` attribute.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        match self {
            IntrospectedName::FromPsl { mapped_name, .. } => mapped_name,
            IntrospectedName::FromSql { .. } => None,
            IntrospectedName::RenamedSanitized { mapped_name } => Some(mapped_name),
        }
    }
}

/// Names that correspond to _enum variants_ in the generated client.
pub(crate) enum EnumVariantName<'a> {
    Empty,
    RenamedSanitized {
        mapped_name: &'a str,
    },
    FromSql {
        name: &'a str,
    },
    FromPsl {
        name: &'a str,
        mapped_name: Option<&'a str>,
    },
}

impl<'a> EnumVariantName<'a> {
    /// Create a name from an SQL identifier.
    pub(crate) fn new_from_sql(name: &'a str) -> Self {
        match name {
            "" => EnumVariantName::Empty,
            _ if crate::sanitize_datamodel_names::needs_sanitation(name) => {
                EnumVariantName::RenamedSanitized { mapped_name: name }
            }
            name => EnumVariantName::FromSql { name },
        }
    }

    /// Output name to the PSL.
    pub(crate) fn prisma_name(&self) -> Cow<'a, str> {
        match self {
            EnumVariantName::Empty => Cow::Borrowed("EMPTY_ENUM_VALUE"),
            EnumVariantName::RenamedSanitized { mapped_name } => {
                crate::sanitize_datamodel_names::sanitize_string(mapped_name)
            }
            EnumVariantName::FromSql { name } | EnumVariantName::FromPsl { name, .. } => Cow::Borrowed(name),
        }
    }

    /// The original name to be used in the `@map` attribute.
    pub(crate) fn mapped_name(&self) -> Option<&'a str> {
        match self {
            EnumVariantName::Empty => Some(""),
            EnumVariantName::RenamedSanitized { mapped_name } => Some(mapped_name),
            EnumVariantName::FromSql { name: _ } => None,
            EnumVariantName::FromPsl { name: _, mapped_name } => *mapped_name,
        }
    }
}
