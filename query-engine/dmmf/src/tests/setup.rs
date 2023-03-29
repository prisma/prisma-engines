use crate::DataModelMetaFormat;

use console::*;
use flate2::*;
use similar::*;
use std::{
    fs::*,
    io::{Read, Write},
};

pub fn write_compressed_snapshot(dmmf: &DataModelMetaFormat, path: &str) -> () {
    let mut encoder = write::GzEncoder::new(File::create(path).unwrap(), Compression::best());
    let json = serde_json::to_vec(dmmf).unwrap();

    encoder.write_all(json.as_slice()).unwrap();
    encoder.finish().unwrap();
}

pub fn read_compressed_snapshot(path: &str) -> serde_json::Value {
    let reader_json = std::fs::read(path).unwrap();
    let mut decoder = read::GzDecoder::new(reader_json.as_slice());
    let mut json: Vec<u8> = vec![];

    decoder.read_to_end(&mut json).unwrap();

    serde_json::from_slice::<serde_json::Value>(&json).unwrap()
}

pub fn panic_with_diff(expected: &str, found: &str) {
    let formatted = format_diff(expected, found);

    panic!(
        r#"
Snapshot comparison failed. Run the test again with UPDATE_EXPECT=1 in the environment to update the snapshot.
======== DIFF ========
{formatted}
======================
Snapshot comparison failed. Run the test again with UPDATE_EXPECT=1 in the environment to update the snapshot.

"#,
    );
}

struct Line(Option<usize>);

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

fn format_diff(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut buf = String::new();

    for (idx, group) in diff.grouped_ops(2).iter().enumerate() {
        if idx > 0 {
            buf.push_str(&format!("{:-^1$}\n", "-", 80));
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, s) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red()),
                    ChangeTag::Insert => ("+", Style::new().green()),
                    ChangeTag::Equal => (" ", Style::new().dim()),
                };

                let index = change.new_index().or_else(|| change.old_index());

                buf.push_str(&format!("{} |{}", style(Line(index)).dim(), s.apply_to(sign).bold()));

                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        buf.push_str(&format!("{}", s.apply_to(value).underlined().bold()));
                    } else {
                        buf.push_str(&format!("{}", s.apply_to(value)));
                    }
                }

                if change.missing_newline() {
                    buf.push('\n');
                }
            }
        }
    }

    buf
}
