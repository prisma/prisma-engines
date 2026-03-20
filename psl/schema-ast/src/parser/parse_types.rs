use super::{Rule, helpers::Pair};
use crate::{ast::*, parser::parse_expression::parse_expression};
use diagnostics::{DatamodelError, Diagnostics, FileId};

pub fn parse_field_type(
    pair: Pair<'_>,
    diagnostics: &mut Diagnostics,
    file_id: FileId,
) -> Result<(FieldArity, FieldType), DatamodelError> {
    assert!(pair.as_rule() == Rule::field_type);
    let current = pair.into_inner().next().unwrap();
    match current.as_rule() {
        Rule::optional_type => Ok((
            FieldArity::Optional,
            parse_base_type(current.into_inner().next().unwrap(), diagnostics, file_id)?,
        )),
        Rule::base_type => Ok((FieldArity::Required, parse_base_type(current, diagnostics, file_id)?)),
        Rule::list_type => Ok((
            FieldArity::List,
            parse_base_type(current.into_inner().next().unwrap(), diagnostics, file_id)?,
        )),
        Rule::legacy_required_type => Err(DatamodelError::new_legacy_parser_error(
            "Fields are required by default, `!` is no longer required.",
            (file_id, current.as_span()).into(),
        )),
        Rule::legacy_list_type => Err(DatamodelError::new_legacy_parser_error(
            "To specify a list, please use `Type[]` instead of `[Type]`.",
            (file_id, current.as_span()).into(),
        )),
        Rule::unsupported_optional_list_type => Err(DatamodelError::new_legacy_parser_error(
            "Optional lists are not supported. Use either `Type[]` or `Type?`.",
            (file_id, current.as_span()).into(),
        )),
        _ => unreachable!("Encountered impossible field during parsing: {:?}", current.tokens()),
    }
}

fn parse_base_type(
    pair: Pair<'_>,
    diagnostics: &mut Diagnostics,
    file_id: FileId,
) -> Result<FieldType, DatamodelError> {
    let current = pair.into_inner().next().unwrap();
    match current.as_rule() {
        Rule::identifier => Ok(FieldType::Supported(Identifier {
            name: current.as_str().to_string(),
            span: Span::from((file_id, current.as_span())),
        })),
        Rule::unsupported_type => match parse_expression(current, diagnostics, file_id) {
            Expression::StringValue(lit, span) => Ok(FieldType::Unsupported(lit, span)),
            _ => unreachable!("Encountered impossible type during parsing"),
        },
        Rule::geometry_type => parse_geometry_type(current, file_id),
        _ => unreachable!("Encountered impossible type during parsing: {:?}", current.tokens()),
    }
}

fn parse_geometry_type(pair: Pair<'_>, file_id: FileId) -> Result<FieldType, DatamodelError> {
    let span = Span::from((file_id, pair.as_span()));
    let mut inner = pair.into_inner();
    let subtype_pair = inner.next().expect("geometry: subtype");
    debug_assert_eq!(subtype_pair.as_rule(), Rule::geometry_subtype);
    let subtype = match subtype_pair.as_str() {
        "Point" => crate::ast::GeometrySubtype::Point,
        "LineString" => crate::ast::GeometrySubtype::LineString,
        "Polygon" => crate::ast::GeometrySubtype::Polygon,
        "MultiPoint" => crate::ast::GeometrySubtype::MultiPoint,
        "MultiLineString" => crate::ast::GeometrySubtype::MultiLineString,
        "MultiPolygon" => crate::ast::GeometrySubtype::MultiPolygon,
        "GeometryCollection" => crate::ast::GeometrySubtype::GeometryCollection,
        "Geometry" => crate::ast::GeometrySubtype::Geometry,
        _ => unreachable!("geometry_subtype rule produced unexpected token"),
    };

    let srid = if let Some(srid_pair) = inner.next() {
        debug_assert_eq!(srid_pair.as_rule(), Rule::geometry_srid);
        let raw = srid_pair.as_str();
        match raw.parse::<i32>() {
            Ok(v) => Some(v),
            Err(_) => {
                return Err(DatamodelError::new_validation_error(
                    "Invalid SRID: expected a valid 32-bit integer.",
                    (file_id, srid_pair.as_span()).into(),
                ));
            }
        }
    } else {
        None
    };

    Ok(FieldType::Geometry { subtype, srid, span })
}
