use crate::{DefaultKind, ValueGenerator};
use prisma_value::PrismaValue;
use psl_core::parser_database::{ast, ScalarType};

pub fn dml_default_kind(default_value: &ast::Expression, scalar_type: Option<ScalarType>) -> DefaultKind {
    // This has all been validated in parser-database, so unwrapping is always safe.
    match default_value {
        ast::Expression::Function(funcname, args, _) if funcname == "dbgenerated" => {
            DefaultKind::Expression(ValueGenerator::new_dbgenerated(
                args.arguments
                    .get(0)
                    .and_then(|arg| arg.value.as_string_value())
                    .map(|(val, _)| val.to_owned())
                    .unwrap_or_else(String::new),
            ))
        }
        ast::Expression::Function(funcname, _, _) if funcname == "auto" => {
            DefaultKind::Expression(ValueGenerator::new_auto())
        }
        ast::Expression::Function(funcname, _args, _) if funcname == "autoincrement" => {
            DefaultKind::Expression(ValueGenerator::new_autoincrement())
        }
        ast::Expression::Function(funcname, _args, _) if funcname == "sequence" => {
            DefaultKind::Expression(ValueGenerator::new_sequence(Vec::new()))
        }
        ast::Expression::Function(funcname, _args, _) if funcname == "uuid" => {
            DefaultKind::Expression(ValueGenerator::new_uuid())
        }
        ast::Expression::Function(funcname, _args, _) if funcname == "cuid" => {
            DefaultKind::Expression(ValueGenerator::new_cuid())
        }
        ast::Expression::Function(funcname, args, _) if funcname == "nanoid" => {
            DefaultKind::Expression(ValueGenerator::new_nanoid(
                args.arguments
                    .get(0)
                    .and_then(|arg| arg.value.as_numeric_value())
                    .map(|(val, _)| val.parse::<u8>().unwrap()),
            ))
        }
        ast::Expression::Function(funcname, _args, _) if funcname == "now" => {
            DefaultKind::Expression(ValueGenerator::new_now())
        }
        ast::Expression::NumericValue(num, _) => match scalar_type {
            Some(ScalarType::Int) => DefaultKind::Single(PrismaValue::Int(num.parse().unwrap())),
            Some(ScalarType::BigInt) => DefaultKind::Single(PrismaValue::BigInt(num.parse().unwrap())),
            Some(ScalarType::Float) => DefaultKind::Single(PrismaValue::Float(num.parse().unwrap())),
            Some(ScalarType::Decimal) => DefaultKind::Single(PrismaValue::Float(num.parse().unwrap())),
            other => unreachable!("{:?}", other),
        },
        ast::Expression::ConstantValue(v, _) => match scalar_type {
            Some(ScalarType::Boolean) => DefaultKind::Single(PrismaValue::Boolean(v.parse().unwrap())),
            None => DefaultKind::Single(PrismaValue::Enum(v.to_owned())),
            other => unreachable!("{:?}", other),
        },
        ast::Expression::StringValue(v, _) => match scalar_type {
            Some(ScalarType::DateTime) => DefaultKind::Single(PrismaValue::DateTime(v.parse().unwrap())),
            Some(ScalarType::String) => DefaultKind::Single(PrismaValue::String(v.parse().unwrap())),
            Some(ScalarType::Json) => DefaultKind::Single(PrismaValue::Json(v.parse().unwrap())),
            Some(ScalarType::Decimal) => DefaultKind::Single(PrismaValue::Float(v.parse().unwrap())),
            Some(ScalarType::Bytes) => DefaultKind::Single(PrismaValue::Bytes(prisma_value::decode_bytes(v).unwrap())),
            other => unreachable!("{:?}", other),
        },
        ast::Expression::Array(values, _) => {
            let values = values
                .iter()
                .map(|expr| dml_default_kind(expr, scalar_type).unwrap_single())
                .collect();

            DefaultKind::Single(PrismaValue::List(values))
        }
        other => unreachable!("{:?}", other),
    }
}
