use std::fmt::{Display, Write};

use super::{
    Attribute, Comment, Identifier, Span, WithAttributes, WithDocumentation, WithIdentifier, WithName, WithSpan,
};

/// OGC / PostGIS geometry subtype written in `Geometry(...)` field types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeometrySubtype {
    Point,
    LineString,
    Polygon,
    MultiPoint,
    MultiLineString,
    MultiPolygon,
    GeometryCollection,
    Geometry,
}

impl GeometrySubtype {
    /// PSL spelling of the subtype (e.g. `Point`).
    pub fn as_str(self) -> &'static str {
        match self {
            GeometrySubtype::Point => "Point",
            GeometrySubtype::LineString => "LineString",
            GeometrySubtype::Polygon => "Polygon",
            GeometrySubtype::MultiPoint => "MultiPoint",
            GeometrySubtype::MultiLineString => "MultiLineString",
            GeometrySubtype::MultiPolygon => "MultiPolygon",
            GeometrySubtype::GeometryCollection => "GeometryCollection",
            GeometrySubtype::Geometry => "Geometry",
        }
    }
}

/// A field definition in a model or a composite type.
#[derive(Debug, Clone)]
pub struct Field {
    /// The field's type.
    ///
    /// ```ignore
    /// name String
    ///      ^^^^^^
    /// ```
    pub field_type: FieldType,
    /// The name of the field.
    ///
    /// ```ignore
    /// name String
    /// ^^^^
    /// ```
    pub(crate) name: Identifier,
    /// The arity of the field.
    pub arity: FieldArity,
    /// The attributes of this field.
    ///
    /// ```ignore
    /// name String @id @default("lol")
    ///             ^^^^^^^^^^^^^^^^^^^
    /// ```
    pub attributes: Vec<Attribute>,
    /// The comments for this field.
    ///
    /// ```ignore
    /// /// Lorem ipsum
    ///     ^^^^^^^^^^^
    /// name String @id @default("lol")
    /// ```
    pub(crate) documentation: Option<Comment>,
    /// The location of this field in the text representation.
    pub(crate) span: Span,
}

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let extension = if self.arity.is_list() {
            "[]"
        } else if self.arity.is_optional() {
            "?"
        } else {
            ""
        };

        write!(f, "{} {}{}", self.name(), self.field_type, extension)
    }
}

impl Field {
    /// Finds the position span of the argument in the given field attribute.
    pub fn span_for_argument(&self, attribute: &str, argument: &str) -> Option<Span> {
        self.attributes
            .iter()
            .filter(|a| a.name() == attribute)
            .flat_map(|a| a.arguments.iter())
            .filter(|a| a.name.as_ref().map(|n| n.name.as_str()) == Some(argument))
            .map(|a| a.span)
            .next()
    }

    /// Finds the position span of the given attribute.
    pub fn span_for_attribute(&self, attribute: &str) -> Option<Span> {
        self.attributes
            .iter()
            .filter(|a| a.name() == attribute)
            .map(|a| a.span)
            .next()
    }

    /// The name of the field
    pub fn name(&self) -> &str {
        &self.name.name
    }
}

impl WithIdentifier for Field {
    fn identifier(&self) -> &Identifier {
        &self.name
    }
}

impl WithSpan for Field {
    fn span(&self) -> Span {
        self.span
    }
}

impl WithAttributes for Field {
    fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }
}

impl WithDocumentation for Field {
    fn documentation(&self) -> Option<&str> {
        self.documentation.as_ref().map(|doc| doc.text.as_str())
    }
}

/// An arity of a data model field.
#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldArity {
    /// The field either must be in an insert statement, or the field must have
    /// a default value for the insert to succeed.
    ///
    /// ```ignore
    /// name String
    /// ```
    Required,
    /// The field does not need to be in an insert statement for the write to
    /// succeed.
    ///
    /// ```ignore
    /// name String?
    /// ```
    Optional,
    /// The field can have multiple values stored in the same column.
    ///
    /// ```ignore
    /// name String[]
    /// ```
    List,
}

impl FieldArity {
    pub fn is_list(&self) -> bool {
        matches!(self, &FieldArity::List)
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, &FieldArity::Optional)
    }

    pub fn is_required(&self) -> bool {
        matches!(self, &FieldArity::Required)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Supported(Identifier),
    /// `Geometry(Point, 4326)` or `Geometry(LineString)` (SRID optional).
    Geometry {
        subtype: GeometrySubtype,
        srid: Option<i32>,
        span: Span,
    },
    /// Unsupported("...")
    Unsupported(String, Span),
}

impl FieldType {
    pub fn span(&self) -> Span {
        match self {
            FieldType::Supported(ident) => ident.span,
            FieldType::Geometry { span, .. } => *span,
            FieldType::Unsupported(_, span) => *span,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            FieldType::Supported(supported) => &supported.name,
            FieldType::Geometry { .. } => "Geometry",
            FieldType::Unsupported(name, _) => name,
        }
    }

    pub fn as_unsupported(&self) -> Option<(&str, &Span)> {
        match self {
            FieldType::Unsupported(name, span) => Some((name, span)),
            FieldType::Supported(_) | FieldType::Geometry { .. } => None,
        }
    }
}

impl Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::Supported(ident) => f.write_str(&ident.name),
            FieldType::Geometry { subtype, srid, .. } => {
                f.write_str("Geometry(")?;
                f.write_str(subtype.as_str())?;
                if let Some(srid) = srid {
                    write!(f, ", {srid}")?;
                }
                f.write_char(')')
            }
            FieldType::Unsupported(name, _) => write!(f, "Unsupported({})", crate::string_literal(name)),
        }
    }
}
