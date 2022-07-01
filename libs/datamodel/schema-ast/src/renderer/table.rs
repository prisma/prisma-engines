use super::LineWriteable;
use std::cmp::max;

const COLUMN_SPACING: usize = 1;

#[derive(Debug)]
pub enum Row {
    // the 2nd String is an arbitrary String that does not influence the table layout. We use it for end of line comments.
    /// A row with columns aligned by the table format.
    Regular(Vec<String>, String),
    /// A row without columns.
    Interleaved(String),
}

impl Row {
    fn new_regular() -> Row {
        Row::Regular(Vec::new(), String::new())
    }
}

#[derive(Debug, Default)]
pub(crate) struct TableFormat {
    pub table: Vec<Row>,
}

impl TableFormat {
    fn reset(&mut self) {
        std::mem::take(self);
    }

    pub(crate) fn column_locked_writer_for(&mut self, index: usize) -> ColumnLockedWriter<'_> {
        ColumnLockedWriter {
            formatter: self,
            column: index,
        }
    }

    pub(crate) fn interleave(&mut self, text: &str) {
        self.table.push(Row::Interleaved(String::from(text)));
    }

    // Safely appends to the column with the given index.
    fn append_to(&mut self, text: &str, index: usize) {
        match self.table.last_mut() {
            Some(Row::Regular(row, _)) => {
                while row.len() <= index {
                    row.push(String::new());
                }

                if row[index].is_empty() {
                    row[index] = String::from(text);
                } else {
                    row[index].push_str(text);
                }
            }
            Some(Row::Interleaved(_)) => panic!("Cannot append to col in interleaved mode"),
            None => unreachable!(),
        }
    }

    pub(crate) fn append_suffix_to_current_row(&mut self, text: &str) {
        match self.table.last_mut() {
            Some(Row::Regular(_, suffix)) => suffix.push_str(text),
            _ => panic!("State error: Not inside a regular table row."),
        }
    }

    pub(crate) fn start_new_line(&mut self) {
        self.table.push(Row::new_regular());
    }

    pub(crate) fn render(&mut self, target: &mut super::Renderer) {
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
                        target.write(col);
                        target.write(&" ".repeat(spacing));
                    }
                    if !suffix.is_empty() {
                        if !row.is_empty() {
                            target.write(" ");
                        }
                        target.write(suffix);
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
        let trimmed = text.trim();

        match self.table.last_mut() {
            Some(Row::Regular(row, _)) => row.push(String::from(trimmed)),
            _ => panic!("State error: Not inside a regular table row."),
        }
    }

    fn end_line(&mut self) {}
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
        panic!("Lines cannot be ended from ColumnLockedWriter");
    }
}
