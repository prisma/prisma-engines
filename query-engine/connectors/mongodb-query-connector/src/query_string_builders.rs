//! query_strings provides of types to build strings representing textual mongodb queries
//! from driver types such as Document. These are used for logging / debugging purposes
//! mainly and generated lazily. There is a type of each different type of query to generate
//!
//! All types implemen the QueryStringBuilder trait which is dynamically dispatched to
//! a specific query string builder.
use mongodb::bson::{Bson, Document};
use std::fmt::Write;

pub(crate) trait QueryStringBuilder: Sync + Send {
    fn build(&self) -> String {
        let mut buffer = String::new();

        write!(&mut buffer, "db.{}.{}(", self.collection(), self.query_type()).unwrap();
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
        let stages: Vec<_> = self.stages.iter().map(|stage| Bson::Document(stage.clone())).collect();

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

pub(crate) struct UpdateMany<'a> {
    filter: &'a Document,
    update_docs: &'a [Document],
    coll_name: &'a str,
}

impl UpdateMany<'_> {
    pub(crate) fn new<'a>(filter: &'a Document, update_docs: &'a [Document], coll_name: &'a str) -> UpdateMany<'a> {
        UpdateMany {
            filter,
            update_docs,
            coll_name,
        }
    }
}

impl QueryStringBuilder for UpdateMany<'_> {
    fn collection(&self) -> &str {
        self.coll_name
    }

    fn query_type(&self) -> &str {
        "updateMany"
    }

    fn write_query(&self, buffer: &mut String) {
        fmt_doc(buffer, self.filter, 1).unwrap();

        if cfg!(debug_assertions) {
            writeln!(buffer, ", [").unwrap();
        } else {
            write!(buffer, ", [").unwrap();
        }

        if let Some((last, docs)) = self.update_docs.split_last() {
            for doc in docs {
                fmt_doc(buffer, doc, 1).unwrap();
                writeln!(buffer, ",").unwrap();
            }
            fmt_doc(buffer, last, 1).unwrap();
        }
    }
}

pub(crate) struct UpdateOne<'a> {
    filter: &'a Document,
    update_doc: &'a Document,
    coll_name: &'a str,
}

impl UpdateOne<'_> {
    pub(crate) fn new<'a>(filter: &'a Document, update_doc: &'a Document, coll_name: &'a str) -> UpdateOne<'a> {
        UpdateOne {
            filter,
            update_doc,
            coll_name,
        }
    }
}

impl QueryStringBuilder for UpdateOne<'_> {
    fn collection(&self) -> &str {
        self.coll_name
    }

    fn query_type(&self) -> &str {
        "updateOne"
    }

    fn write_query(&self, buffer: &mut String) {
        fmt_doc(buffer, self.filter, 1).unwrap();

        if cfg!(debug_assertions) {
            writeln!(buffer, ", [").unwrap();
        } else {
            write!(buffer, ", [").unwrap();
        }

        fmt_doc(buffer, self.update_doc, 1).unwrap();
    }
}

pub(crate) struct DeleteMany<'a> {
    filter: &'a Document,
    coll_name: &'a str,
}

impl DeleteMany<'_> {
    pub(crate) fn new<'a>(filter: &'a Document, coll_name: &'a str) -> DeleteMany<'a> {
        DeleteMany { filter, coll_name }
    }
}

impl QueryStringBuilder for DeleteMany<'_> {
    fn collection(&self) -> &str {
        self.coll_name
    }

    fn query_type(&self) -> &str {
        "deleteMany"
    }

    fn write_query(&self, buffer: &mut String) {
        fmt_doc(buffer, self.filter, 1).unwrap();
    }
}

pub(crate) struct InsertMany<'a> {
    insert_docs: &'a [Document],
    coll_name: &'a str,
    ordered: bool,
}

impl InsertMany<'_> {
    pub(crate) fn new<'a>(insert_docs: &'a [Document], ordered: bool, coll_name: &'a str) -> InsertMany<'a> {
        InsertMany {
            insert_docs,
            coll_name,
            ordered,
        }
    }
}

impl QueryStringBuilder for InsertMany<'_> {
    fn collection(&self) -> &str {
        self.coll_name
    }

    fn query_type(&self) -> &str {
        "insertMany"
    }

    fn write_query(&self, buffer: &mut String) {
        for doc in self.insert_docs {
            fmt_doc(buffer, doc, 1).unwrap();
        }

        write!(buffer, "], ").unwrap();
        write!(buffer, r#"{{ "ordered": {} }}"#, self.ordered).unwrap();
    }
}

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
