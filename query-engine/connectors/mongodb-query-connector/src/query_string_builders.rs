//! query_strings provides functions for building query strings aynchronously from values
//! of driver types such as Document. These query strings are not feed trough the wire to
//! mongodb their main purpose is to add the query to log traces
use mongodb::bson::{Bson, Document};
use std::fmt::Write;

macro_rules! write_indented {
    ($buffer:expr, $depth:expr, $fmt_str:literal, $($args:expr)*) => {
        write!($buffer, "{}{}", indent($depth), format!($fmt_str, $($args)*))?;
    };
}

pub(crate) trait QueryStringBuilder: Sync + Send {
    fn build(&self) -> String {
        let mut buffer = String::new();

        write!(&mut buffer, "db.{}.{}", self.collection(), self.query_type()).unwrap();
        self.write_query(&mut buffer);
        write!(&mut buffer, ")").unwrap();

        buffer
    }

    fn collection(&self) -> &str;
    fn query_type(&self) -> &str;
    fn write_query(&self, buffer: &mut String);
}

pub(crate) struct Aggregate<'a> {
    stages: &'a [Document],
    coll_name: &'a str,
}

impl Aggregate<'_> {
    pub(crate) fn new<'a>(stages: &'a [Document], coll_name: &'a str) -> Aggregate<'a> {
        Aggregate { stages, coll_name }
    }
}

impl QueryStringBuilder for Aggregate<'_> {
    fn collection(&self) -> &str {
        self.coll_name
    }

    fn query_type(&self) -> &str {
        "aggregate"
    }

    fn write_query(&self, buffer: &mut String) {
        let stages: Vec<_> = self
            .stages
            .into_iter()
            .map(|stage| Bson::Document(stage.clone()))
            .collect();

        fmt_list(buffer, &stages, 1).unwrap();
    }
}

pub(crate) struct InsertOne<'a> {
    doc: &'a Document,
    coll_name: &'a str,
}

impl InsertOne<'_> {
    pub(crate) fn new<'a>(doc: &'a Document, coll_name: &'a str) -> InsertOne<'a> {
        InsertOne { doc, coll_name }
    }
}

impl QueryStringBuilder for InsertOne<'_> {
    fn collection(&self) -> &str {
        self.coll_name
    }

    fn query_type(&self) -> &str {
        "insertOne"
    }

    fn write_query(&self, buffer: &mut String) {
        fmt_doc(buffer, self.doc, 1).unwrap();
    }
}

#[cfg(debug_assertions)]
fn indent(depth: usize) -> String {
    " ".repeat(4 * depth)
}

#[cfg(not(debug_assertions))]
fn indent(_: usize) -> String {
    String::from(" ")
}

fn fmt_doc(buffer: &mut String, doc: &Document, depth: usize) -> std::fmt::Result {
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

fn fmt_list(buffer: &mut String, list: &[Bson], depth: usize) -> std::fmt::Result {
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
