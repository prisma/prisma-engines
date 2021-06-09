use super::MongoReadQuery;
use itertools::Itertools;
use mongodb::{
    bson::{Bson, Document},
    options::FindOptions,
};
use std::fmt::Write;

// Bare-bones logging impl for debugging.
pub fn log_query(coll_name: &str, query: &MongoReadQuery) {
    let mut buffer = String::new();
    fmt_query(&mut buffer, coll_name, query).unwrap();

    tracing::debug!("{}", buffer);
}

macro_rules! write_indented {
    ($buffer:expr, $depth:expr, $fmt_str:literal, $($args:expr)*) => {
        write!($buffer, "{}{}", indent($depth), format!($fmt_str, $($args)*))?;
    };
}

fn fmt_query(buffer: &mut String, coll_name: &str, query: &MongoReadQuery) -> std::fmt::Result {
    match query {
        MongoReadQuery::Find(find) => {
            write!(buffer, "db.{}.find(", coll_name)?;

            if let Some(ref filter) = find.filter {
                fmt_doc(buffer, filter, 1)?;
                write!(buffer, ", ")?;
            }

            fmt_opts(buffer, &find.options, 1)?;
            write!(buffer, ")")
        }
        MongoReadQuery::Pipeline(pipeline) => {
            write!(buffer, "db.{}.aggregate(", coll_name)?;

            let stages: Vec<_> = pipeline
                .stages
                .iter()
                .map(|stage| Bson::Document(stage.clone()))
                .collect();

            fmt_list(buffer, &stages, 1)?;
            write!(buffer, ")")
        }
    }
}

fn fmt_opts(buffer: &mut String, opts: &FindOptions, depth: usize) -> std::fmt::Result {
    writeln!(buffer, "{{")?;

    if let Some(skip) = opts.skip {
        write_indented!(buffer, depth, "skip: {},\n", skip);
    }

    if let Some(limit) = opts.limit {
        write_indented!(buffer, depth, "limit: {},\n", limit);
    }

    if let Some(ref sort) = opts.sort {
        write_indented!(buffer, depth, "sort: ",);
        fmt_doc(buffer, sort, depth + 1)?;
        writeln!(buffer, ",")?;
    }

    if let Some(ref projection) = opts.projection {
        write_indented!(buffer, depth, "projection: ",);
        fmt_doc(buffer, projection, depth + 1)?;
        writeln!(buffer)?;
    }

    write!(buffer, "}}")
}

fn indent(depth: usize) -> String {
    " ".repeat(4 * depth)
}

fn fmt_doc(buffer: &mut String, doc: &Document, depth: usize) -> std::fmt::Result {
    writeln!(buffer, "{{")?;

    for (key, value) in doc {
        write_indented!(buffer, depth, "{}: ", key);
        fmt_val(buffer, value, depth)?;
        writeln!(buffer, ",")?;
    }

    write_indented!(buffer, usize::max(depth - 1, 0), "}}",);
    Ok(())
}

fn fmt_list(buffer: &mut String, list: &[Bson], depth: usize) -> std::fmt::Result {
    writeln!(buffer, "[")?;

    for item in list {
        write_indented!(buffer, depth, "",);
        fmt_val(buffer, item, depth)?;
        writeln!(buffer, ",")?;
    }

    write_indented!(buffer, usize::max(depth - 1, 0), "]",);
    Ok(())
}

fn fmt_val(buffer: &mut String, val: &Bson, depth: usize) -> std::fmt::Result {
    match val {
        Bson::ObjectId(oid) => write!(buffer, "\"{}\"", oid),
        Bson::Double(d) => write!(buffer, "{}", d),
        Bson::String(s) => write!(buffer, "\"{}\"", s),
        Bson::Array(ary) => fmt_list(buffer, ary, depth + 1),
        Bson::Document(doc) => fmt_doc(buffer, doc, depth + 1),
        Bson::Boolean(b) => write!(buffer, "{}", b),
        Bson::Int32(i) => write!(buffer, "{}", i),
        Bson::Int64(i) => write!(buffer, "{}", i),
        Bson::Binary(bin) => write!(buffer, "{:02x}", bin.bytes.iter().format(" ")),
        Bson::DateTime(dt) => write!(buffer, "\"{}\"", dt.to_chrono().to_rfc3339()),
        Bson::Null => write!(buffer, "null"),
        Bson::Undefined => write!(buffer, "undefined"),
        Bson::RegularExpression(reg) => write!(buffer, r#""{}", $options: "{}""#, reg.pattern, reg.options),
        Bson::Decimal128(dec) => write!(buffer, "{}", dec),

        Bson::Symbol(_) => todo!(),
        Bson::Timestamp(_) => todo!(),
        Bson::MaxKey => todo!(),
        Bson::MinKey => todo!(),
        Bson::DbPointer(_) => todo!(),
        Bson::JavaScriptCode(_) => todo!(),
        Bson::JavaScriptCodeWithScope(_) => todo!(),
    }
}
