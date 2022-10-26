use mongodb::bson::{Bson, Document};
use std::fmt::Write;
use tracing::debug;

macro_rules! write_indented {
    ($buffer:expr, $depth:expr, $fmt_str:literal, $($args:expr)*) => {
        write!($buffer, "{}{}", indent($depth), format!($fmt_str, $($args)*))?;
    };
}

#[cfg(debug_assertions)]
fn indent(depth: usize) -> String {
    " ".repeat(4 * depth)
}

#[cfg(not(debug_assertions))]
fn indent(_: usize) -> String {
    String::from(" ")
}

pub(crate) fn fmt_doc(buffer: &mut String, doc: &Document, depth: usize) -> std::fmt::Result {
    if cfg!(debug_assertions) {
        writeln!(buffer, "{{")?;
    } else {
        write!(buffer, "{{")?;
    }

    for (key, value) in doc {
        write_indented!(buffer, depth, "{}: ", key);
        fmt_val(buffer, value, depth)?;
        if cfg!(debug_assertions) {
            writeln!(buffer, ",")?;
        } else {
            write!(buffer, ",")?;
        }
    }

    write_indented!(buffer, usize::max(depth - 1, 0), "}}",);
    Ok(())
}

pub(crate) fn fmt_list(buffer: &mut String, list: &[Bson], depth: usize) -> std::fmt::Result {
    if cfg!(debug_assertions) {
        writeln!(buffer, "[")?;
    } else {
        write!(buffer, "[")?;
    }

    for item in list {
        write_indented!(buffer, depth, "",);
        fmt_val(buffer, item, depth)?;
        if cfg!(debug_assertions) {
            writeln!(buffer, ",")?;
        } else {
            write!(buffer, ",")?;
        }
    }

    write_indented!(buffer, usize::max(depth - 1, 0), "]",);
    Ok(())
}

fn fmt_val(buffer: &mut String, val: &Bson, depth: usize) -> std::fmt::Result {
    match val {
        Bson::Array(ary) => fmt_list(buffer, ary, depth + 1),
        Bson::Document(doc) => fmt_doc(buffer, doc, depth + 1),
        val => write!(buffer, "{}", val),
    }
}

pub(crate) fn log_update_many_vec(coll: &str, filter: &Document, docs: &[Document]) {
    let mut buffer = String::new();

    write!(&mut buffer, "db.{}.updateMany(", coll).unwrap();
    fmt_doc(&mut buffer, filter, 1).unwrap();

    if cfg!(debug_assertions) {
        writeln!(&mut buffer, ", [").unwrap();
    } else {
        write!(&mut buffer, ", [").unwrap();
    }

    if let Some((last, docs)) = docs.split_last() {
        for doc in docs {
            fmt_doc(&mut buffer, doc, 1).unwrap();
            writeln!(&mut buffer, ",").unwrap();
        }
        fmt_doc(&mut buffer, last, 1).unwrap();
    }

    write!(&mut buffer, "])").unwrap();

    let params: Vec<i32> = Vec::new();
    debug!(target: "mongodb_query_connector::query", query = %buffer, item_type = "query", is_query = true, params = ?params);
}

pub(crate) fn log_update_many(coll: &str, filter: &Document, doc: &Document) {
    let mut buffer = String::new();

    write!(&mut buffer, "db.{}.updateMany(", coll).unwrap();
    fmt_doc(&mut buffer, filter, 1).unwrap();

    if cfg!(debug_assertions) {
        writeln!(&mut buffer, ", ").unwrap();
    } else {
        write!(&mut buffer, ", ").unwrap();
    }

    fmt_doc(&mut buffer, doc, 1).unwrap();
    write!(&mut buffer, ")").unwrap();
}

pub(crate) fn log_update_one(coll: &str, filter: &Document, doc: &Document) {
    let mut buffer = String::new();

    write!(&mut buffer, "db.{}.updateOne(", coll).unwrap();
    fmt_doc(&mut buffer, filter, 1).unwrap();

    if cfg!(debug_assertions) {
        writeln!(&mut buffer, ", ").unwrap();
    } else {
        write!(&mut buffer, ", ").unwrap();
    }

    fmt_doc(&mut buffer, doc, 1).unwrap();
    write!(&mut buffer, ")").unwrap();

    let params: Vec<i32> = Vec::new();
    debug!(target: "mongodb_query_connector::query", query = %buffer, item_type = "query", is_query = true, params = ?params);
}

pub(crate) fn log_delete_many(coll: &str, filter: &Document) {
    let mut buffer = String::new();

    write!(&mut buffer, "db.{}.deleteMany(", coll).unwrap();
    fmt_doc(&mut buffer, filter, 1).unwrap();
    write!(&mut buffer, ")").unwrap();

    let params: Vec<i32> = Vec::new();
    debug!(target: "mongodb_query_connector::query", query = %buffer, item_type = "query", is_query = true, params = ?params);
}

pub(crate) fn log_insert_many(coll: &str, docs: &[Document], ordered: bool) {
    let mut buffer = String::new();

    write!(&mut buffer, "db.{}.insertMany(", coll).unwrap();

    for doc in docs {
        fmt_doc(&mut buffer, doc, 1).unwrap();
    }

    write!(&mut buffer, "], ").unwrap();
    write!(&mut buffer, r#"{{ "ordered": {} }}"#, ordered).unwrap();

    let params: Vec<i32> = Vec::new();
    debug!(target: "mongodb_query_connector::query", query = %buffer, item_type = "query", is_query = true, params = ?params);
}
