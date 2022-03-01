use super::LineWriteable;
use super::StringBuilder;
use std::cmp::max;

const COLUMN_SPACING: usize = 1;

#[derive(Debug)]
pub enum Row {
    // the 2nd String is an arbitrary String that does not influence the table layout. We use it for end of line comments.
    Regular(Vec<String>, String),
    Interleaved(String),
}

impl Row {
    fn new_regular() -> Row {
        Row::Regular(Vec::new(), "".to_owned())
    }
}

#[derive(Debug)]
pub struct TableFormat {
    pub table: Vec<Row>,
    row: i32,
    line_ending: bool,
    maybe_new_line: bool,
}

impl TableFormat {
    pub fn new() -> TableFormat {
        TableFormat {
            table: Vec::new(),
            row: -1,
            line_ending: true,
            maybe_new_line: false,
        }
    }

    fn reset(&mut self) {
        self.table = Vec::new();
        self.row = -1;
        self.line_ending = true;
        self.maybe_new_line = false;
    }

    pub fn interleave_writer(&mut self) -> TableFormatInterleaveWrapper<'_> {
        TableFormatInterleaveWrapper {
            formatter: self,
            string_builder: StringBuilder::new(),
        }
    }

    pub fn column_locked_writer_for(&mut self, index: usize) -> ColumnLockedWriter<'_> {
        ColumnLockedWriter {
            formatter: self,
            column: index,
        }
    }

    pub fn interleave(&mut self, text: &str) {
        self.table.push(Row::Interleaved(String::from(text)));
        // We've just ended a line.
        self.line_ending = false;
        self.maybe_new_line = false;
        self.row += 1;

        // Prepare next new line.
        self.end_line();
    }

    // Safely appends to the column with the given index.
    fn append_to(&mut self, text: &str, index: usize) {
        if self.line_ending || self.maybe_new_line {
            self.start_new_line();
            self.line_ending = false;
            self.maybe_new_line = false;
        }

        match &mut self.table[self.row as usize] {
            Row::Regular(row, _) => {
                while row.len() <= index {
                    row.push(String::new());
                }

                if row[index].is_empty() {
                    row[index] = String::from(text);
                } else {
                    row[index] = format!("{}{}", &row[index], text);
                }
            }
            Row::Interleaved(_) => panic!("Cannot append to col in interleaved mode"),
        }
    }

    pub fn append_suffix_to_current_row(&mut self, text: &str) {
        match &mut self.table[self.row as usize] {
            Row::Regular(_, suffix) => suffix.push_str(text),
            _ => panic!("State error: Not inside a regular table row."),
        }
    }

    pub fn start_new_line(&mut self) {
        self.table.push(Row::new_regular());
        self.row += 1;
    }

    pub fn render(&mut self, target: &mut dyn LineWriteable) {
        // First, measure cols
        let mut max_number_of_columns = 0;

        for row in &self.table {
            if let Row::Regular(row, _) = row {
                max_number_of_columns = max(max_number_of_columns, row.len());
            }
        }

        let mut max_widths_for_each_column = vec![0; max_number_of_columns];

        for row in &self.table {
            if let Row::Regular(row, _) = row {
                for (i, col) in row.iter().enumerate() {
                    max_widths_for_each_column[i] = max(max_widths_for_each_column[i], col.len());
                }
            }
        }

        // Then, render
        for row in &self.table {
            match row {
                Row::Regular(row, suffix) => {
                    for (i, col) in row.iter().enumerate() {
                        let spacing = if i == row.len() - 1 {
                            0 // Do not space last column.
                        } else {
                            max_widths_for_each_column[i] - col.len() + COLUMN_SPACING
                        };
                        target.write(&format!("{}{}", col, " ".repeat(spacing)));
                    }
                    if !suffix.is_empty() {
                        target.write(&format!(" {}", suffix));
                    }
                }
                Row::Interleaved(text) => {
                    target.write(text);
                }
            }

            target.end_line();
        }

        self.reset()
    }
}

impl LineWriteable for TableFormat {
    fn write(&mut self, text: &str) {
        if self.line_ending || self.maybe_new_line {
            self.start_new_line();
            self.line_ending = false;
            self.maybe_new_line = false;
        }

        let trimmed = text.trim();

        match &mut self.table[self.row as usize] {
            Row::Regular(row, _) => row.push(String::from(trimmed)),
            _ => panic!("State error: Not inside a regular table row."),
        }
    }

    fn end_line(&mut self) {
        // Lazy line ending.
        if self.line_ending {
            self.start_new_line();
            self.maybe_new_line = false;
        }

        self.line_ending = true;
    }

    fn line_empty(&self) -> bool {
        self.line_ending
    }

    fn maybe_end_line(&mut self) {
        self.maybe_new_line = true
    }
}

pub struct TableFormatInterleaveWrapper<'a> {
    formatter: &'a mut TableFormat,
    string_builder: StringBuilder,
}

impl<'a> LineWriteable for TableFormatInterleaveWrapper<'a> {
    fn write(&mut self, text: &str) {
        self.string_builder.write(text);
    }

    fn end_line(&mut self) {
        self.formatter.interleave(&self.string_builder.to_string());
        self.string_builder = StringBuilder::new();
    }

    fn maybe_end_line(&mut self) {
        self.formatter.maybe_end_line();
    }

    fn line_empty(&self) -> bool {
        true
    }
}

pub struct ColumnLockedWriter<'a> {
    formatter: &'a mut TableFormat,
    column: usize,
}

impl<'a> LineWriteable for ColumnLockedWriter<'a> {
    fn write(&mut self, text: &str) {
        self.formatter.append_to(text, self.column);
    }

    fn end_line(&mut self) {
        self.formatter.end_line();
    }

    fn maybe_end_line(&mut self) {
        self.formatter.maybe_end_line();
    }

    fn line_empty(&self) -> bool {
        if self.formatter.line_empty() {
            true
        } else {
            match &self.formatter.table.last().unwrap() {
                Row::Regular(row, _) => row.len() <= self.column || row[self.column].is_empty(),
                Row::Interleaved(s) => s.is_empty(),
            }
        }
    }
}

impl Default for TableFormat {
    fn default() -> Self {
        Self::new()
    }
}
