use crate::{ast, coerce, types::SortOrder, DatamodelError};

pub(crate) enum OperatorClass<'a> {
    Constant(crate::OperatorClass),
    Raw(&'a str),
}

impl<'a> From<crate::OperatorClass> for OperatorClass<'a> {
    fn from(inner: crate::OperatorClass) -> Self {
        Self::Constant(inner)
    }
}

#[derive(Default)]
pub(crate) struct IndexFieldAttributes<'a> {
    pub(crate) field_name: &'a str,
    pub(crate) sort_order: Option<SortOrder>,
    pub(crate) length: Option<u32>,
    pub(crate) operator_class: Option<OperatorClass<'a>>,
}

struct FieldArguments<'a> {
    sort_order: Option<SortOrder>,
    length: Option<u32>,
    operator_class: Option<OperatorClass<'a>>,
}

pub(crate) fn coerce_field_array_with_args<'a>(
    expr: &'a ast::Expression,
    diagnostics: &mut diagnostics::Diagnostics,
) -> Option<Vec<IndexFieldAttributes<'a>>> {
    let f = |expr: &'a ast::Expression, diagnostics: &mut diagnostics::Diagnostics| -> Option<_> {
        match expr {
            ast::Expression::ConstantValue(field_name, _) => Some(IndexFieldAttributes {
                field_name,
                ..Default::default()
            }),
            ast::Expression::Function(field_name, args, _) => {
                let args = field_args(&args.arguments, diagnostics);
                let attrs = IndexFieldAttributes {
                    field_name,
                    sort_order: args.sort_order,
                    length: args.length,
                    operator_class: args.operator_class,
                };

                Some(attrs)
            }
            _ => {
                diagnostics.push_error(DatamodelError::new_type_mismatch_error(
                    "constant literal",
                    expr.describe_value_type(),
                    &expr.to_string(),
                    expr.span(),
                ));
                None
            }
        }
    };

    crate::coerce_array(expr, &f, diagnostics)
}

fn field_args<'a>(args: &'a [ast::Argument], diagnostics: &mut diagnostics::Diagnostics) -> FieldArguments<'a> {
    let sort_order = args
        .iter()
        .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("sort"))
        .and_then(|arg| match coerce::constant(&arg.value, diagnostics) {
            Some("Asc") => Some(SortOrder::Asc),
            Some("Desc") => Some(SortOrder::Desc),
            Some(_) => {
                diagnostics.push_error(DatamodelError::new_parser_error("Asc, Desc".to_owned(), arg.span));
                None
            }
            None => None,
        });

    let length = args
        .iter()
        .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("length"))
        .and_then(|arg| coerce::integer(&arg.value, diagnostics))
        .filter(|i| *i >= 0)
        .map(|i| i as u32);

    let operator_class = args
        .iter()
        .find(|arg| arg.name.as_ref().map(|n| n.name.as_str()) == Some("ops"))
        .and_then(|arg| match &arg.value {
            ast::Expression::ConstantValue(s, span) => match s.as_str() {
                // gist
                "InetOps" => Some(OperatorClass::from(crate::OperatorClass::InetOps)),

                // gin
                "JsonbOps" => Some(OperatorClass::from(crate::OperatorClass::JsonbOps)),
                "JsonbPathOps" => Some(OperatorClass::from(crate::OperatorClass::JsonbPathOps)),
                "ArrayOps" => Some(OperatorClass::from(crate::OperatorClass::ArrayOps)),

                // sp-gist
                "TextOps" => Some(OperatorClass::from(crate::OperatorClass::TextOps)),

                // brin
                "BitMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::BitMinMaxOps)),
                "VarBitMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::VarBitMinMaxOps)),
                "BpcharBloomOps" => Some(OperatorClass::from(crate::OperatorClass::BpcharBloomOps)),
                "BpcharMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::BpcharMinMaxOps)),
                "ByteaBloomOps" => Some(OperatorClass::from(crate::OperatorClass::ByteaBloomOps)),
                "ByteaMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::ByteaMinMaxOps)),
                "DateBloomOps" => Some(OperatorClass::from(crate::OperatorClass::DateBloomOps)),
                "DateMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::DateMinMaxOps)),
                "DateMinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::DateMinMaxMultiOps)),
                "Float4BloomOps" => Some(OperatorClass::from(crate::OperatorClass::Float4BloomOps)),
                "Float4MinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::Float4MinMaxOps)),
                "Float4MinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::Float4MinMaxMultiOps)),
                "Float8BloomOps" => Some(OperatorClass::from(crate::OperatorClass::Float8BloomOps)),
                "Float8MinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::Float8MinMaxOps)),
                "Float8MinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::Float8MinMaxMultiOps)),
                "InetInclusionOps" => Some(OperatorClass::from(crate::OperatorClass::InetInclusionOps)),
                "InetBloomOps" => Some(OperatorClass::from(crate::OperatorClass::InetBloomOps)),
                "InetMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::InetMinMaxOps)),
                "InetMinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::InetMinMaxMultiOps)),
                "Int2BloomOps" => Some(OperatorClass::from(crate::OperatorClass::Int2BloomOps)),
                "Int2MinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::Int2MinMaxOps)),
                "Int2MinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::Int2MinMaxMultiOps)),
                "Int4BloomOps" => Some(OperatorClass::from(crate::OperatorClass::Int4BloomOps)),
                "Int4MinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::Int4MinMaxOps)),
                "Int4MinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::Int4MinMaxMultiOps)),
                "Int8BloomOps" => Some(OperatorClass::from(crate::OperatorClass::Int8BloomOps)),
                "Int8MinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::Int8MinMaxOps)),
                "Int8MinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::Int8MinMaxMultiOps)),
                "NumericBloomOps" => Some(OperatorClass::from(crate::OperatorClass::NumericBloomOps)),
                "NumericMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::NumericMinMaxOps)),
                "NumericMinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::NumericMinMaxMultiOps)),
                "OidBloomOps" => Some(OperatorClass::from(crate::OperatorClass::OidBloomOps)),
                "OidMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::OidMinMaxOps)),
                "OidMinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::OidMinMaxMultiOps)),
                "TextBloomOps" => Some(OperatorClass::from(crate::OperatorClass::TextBloomOps)),
                "TextMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::TextMinMaxOps)),
                "TimestampBloomOps" => Some(OperatorClass::from(crate::OperatorClass::TimestampBloomOps)),
                "TimestampMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::TimestampMinMaxOps)),
                "TimestampMinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::TimestampMinMaxMultiOps)),
                "TimestampTzBloomOps" => Some(OperatorClass::from(crate::OperatorClass::TimestampTzBloomOps)),
                "TimestampTzMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::TimestampTzMinMaxOps)),
                "TimestampTzMinMaxMultiOps" => {
                    Some(OperatorClass::from(crate::OperatorClass::TimestampTzMinMaxMultiOps))
                }
                "TimeBloomOps" => Some(OperatorClass::from(crate::OperatorClass::TimeBloomOps)),
                "TimeMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::TimeMinMaxOps)),
                "TimeMinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::TimeMinMaxMultiOps)),
                "TimeTzBloomOps" => Some(OperatorClass::from(crate::OperatorClass::TimeTzBloomOps)),
                "TimeTzMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::TimeTzMinMaxOps)),
                "TimeTzMinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::TimeTzMinMaxMultiOps)),
                "UuidBloomOps" => Some(OperatorClass::from(crate::OperatorClass::UuidBloomOps)),
                "UuidMinMaxOps" => Some(OperatorClass::from(crate::OperatorClass::UuidMinMaxOps)),
                "UuidMinMaxMultiOps" => Some(OperatorClass::from(crate::OperatorClass::UuidMinMaxMultiOps)),

                s => {
                    diagnostics.push_error(DatamodelError::new_parser_error(
                        format!("Invalid operator class: {s}"),
                        *span,
                    ));
                    None
                }
            },
            ast::Expression::Function(fun, args, span) => match fun.as_str() {
                "raw" => match args.arguments.as_slice() {
                    [arg] => match &arg.value {
                        ast::Expression::StringValue(s, _) => Some(OperatorClass::Raw(s.as_str())),
                        _ => {
                            diagnostics.push_error(DatamodelError::new_parser_error(
                                "Invalid parameter type: expected string".into(),
                                *span,
                            ));
                            None
                        }
                    },
                    args => {
                        diagnostics.push_error(DatamodelError::new_parser_error(
                            format!("Wrong number of arguments. Expected: 1, got: {}", args.len()),
                            *span,
                        ));
                        None
                    }
                },
                _ => panic!(),
            },
            _ => {
                diagnostics.push_error(DatamodelError::new_parser_error("operator class".to_owned(), arg.span));
                None
            }
        });

    FieldArguments {
        sort_order,
        length,
        operator_class,
    }
}
