use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, Utc};
use quaint::{
    prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily},
    visitor::Visitor,
};
use query_structure::{ModelProjection, PrismaValue};
use sql_query_connector::{context::Context, model_extensions::AsColumns, query_builder};

use crate::{
    compiler::expression::{DbQuery, Expression},
    Query, ReadQuery,
};

use super::TranslateResult;

pub(crate) fn translate_query(query: Query) -> TranslateResult<Expression> {
    let connection_info = ConnectionInfo::External(ExternalConnectionInfo::new(
        SqlFamily::Postgres,
        "public".to_owned(),
        None,
    ));

    let ctx = Context::new(&connection_info, None);

    match query {
        Query::Read(rq) => translate_read_query(rq, &ctx),
        _ => unimplemented!(),
    }
}

fn translate_read_query(query: ReadQuery, ctx: &Context<'_>) -> TranslateResult<Expression> {
    let select = match query {
        ReadQuery::RecordQuery(rq) => {
            let selected_fields = rq.selected_fields.without_relations().into_virtuals_last();
            query_builder::read::get_records(
                &rq.model,
                ModelProjection::from(&selected_fields)
                    .as_columns(ctx)
                    .mark_all_selected(),
                selected_fields.virtuals(),
                rq.filter.expect("ReadOne query should always have filter set"),
                ctx,
            )
            .limit(1)
        }

        ReadQuery::ManyRecordsQuery(mrq) => {
            let selected_fields = mrq.selected_fields.without_relations().into_virtuals_last();

            // TODO: we ignore chunking for now
            query_builder::read::get_records(
                &mrq.model,
                ModelProjection::from(&selected_fields)
                    .as_columns(ctx)
                    .mark_all_selected(),
                selected_fields.virtuals(),
                mrq.args,
                ctx,
            )
        }

        _ => unimplemented!(),
    };

    let db_query = build_db_query(select)?;

    Ok(Expression::ReadQuery(db_query))
}

fn build_db_query<'a>(query: impl Into<quaint::ast::Query<'a>>) -> TranslateResult<DbQuery> {
    let (sql, params) = quaint::visitor::Postgres::build(query)?;
    let params = params.into_iter().map(quaint_value_to_prisma_value).collect::<Vec<_>>();
    Ok(DbQuery::new(sql, params))
}

fn quaint_value_to_prisma_value(value: quaint::Value<'_>) -> PrismaValue {
    match value.typed {
        quaint::ValueType::Int32(Some(i)) => PrismaValue::Int(i.into()),
        quaint::ValueType::Int32(None) => PrismaValue::Null,
        quaint::ValueType::Int64(Some(i)) => PrismaValue::BigInt(i),
        quaint::ValueType::Int64(None) => PrismaValue::Null,
        quaint::ValueType::Float(Some(f)) => PrismaValue::Float(
            BigDecimal::from_f32(f)
                .expect("float to decimal conversion should succeed")
                .normalized(),
        ),
        quaint::ValueType::Float(None) => PrismaValue::Null,
        quaint::ValueType::Double(Some(d)) => PrismaValue::Float(
            BigDecimal::from_f64(d)
                .expect("double to decimal conversion should succeed")
                .normalized(),
        ),
        quaint::ValueType::Double(None) => PrismaValue::Null,
        quaint::ValueType::Text(Some(s)) => PrismaValue::String(s.into_owned()),
        quaint::ValueType::Text(None) => PrismaValue::Null,
        quaint::ValueType::Enum(Some(e), _) => PrismaValue::Enum(e.into_owned()),
        quaint::ValueType::Enum(None, _) => PrismaValue::Null,
        quaint::ValueType::EnumArray(Some(es), _) => PrismaValue::List(
            es.into_iter()
                .map(|e| e.into_text())
                .map(quaint_value_to_prisma_value)
                .collect(),
        ),
        quaint::ValueType::EnumArray(None, _) => PrismaValue::Null,
        quaint::ValueType::Bytes(Some(b)) => PrismaValue::Bytes(b.into_owned()),
        quaint::ValueType::Bytes(None) => PrismaValue::Null,
        quaint::ValueType::Boolean(Some(b)) => PrismaValue::Boolean(b),
        quaint::ValueType::Boolean(None) => PrismaValue::Null,
        quaint::ValueType::Char(Some(c)) => PrismaValue::String(c.to_string()),
        quaint::ValueType::Char(None) => PrismaValue::Null,
        quaint::ValueType::Array(Some(a)) => {
            PrismaValue::List(a.into_iter().map(quaint_value_to_prisma_value).collect())
        }
        quaint::ValueType::Array(None) => PrismaValue::Null,
        quaint::ValueType::Numeric(Some(bd)) => PrismaValue::Float(bd),
        quaint::ValueType::Numeric(None) => PrismaValue::Null,
        quaint::ValueType::Json(Some(j)) => PrismaValue::Json(j.to_string()),
        quaint::ValueType::Json(None) => PrismaValue::Null,
        quaint::ValueType::Xml(Some(x)) => PrismaValue::String(x.into_owned()),
        quaint::ValueType::Xml(None) => PrismaValue::Null,
        quaint::ValueType::Uuid(Some(u)) => PrismaValue::Uuid(u),
        quaint::ValueType::Uuid(None) => PrismaValue::Null,
        quaint::ValueType::DateTime(Some(dt)) => PrismaValue::DateTime(dt.into()),
        quaint::ValueType::DateTime(None) => PrismaValue::Null,
        quaint::ValueType::Date(Some(d)) => {
            let dt = DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap(), Utc);
            PrismaValue::DateTime(dt.into())
        }
        quaint::ValueType::Date(None) => PrismaValue::Null,
        quaint::ValueType::Time(Some(t)) => {
            let d = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let dt = DateTime::<Utc>::from_naive_utc_and_offset(d.and_time(t), Utc);
            PrismaValue::DateTime(dt.into())
        }
        quaint::ValueType::Time(None) => PrismaValue::Null,
    }
}
