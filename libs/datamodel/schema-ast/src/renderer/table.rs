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

#[derive(Debug, Default)]
pub(crate) struct TableFormat {
    pub table: Vec<Row>,
}

impl TableFormat {
    fn reset(&mut self) {
        std::mem::take(self);
    }

    pub(crate) fn column_locked_writer_for(&mut self, index: usize) -> &mut String {
        match self.table.last_mut().unwrap() {
            Row::Interleaved(row) => row,
            Row::Regular(columns, _) => {
                columns.resize(index + 1, String::new());
                &mut columns[index]
            }
        }
    }

    pub(crate) fn interleave(&mut self, text: &str) {
        self.table.push(Row::Interleaved(String::from(text)));
    }

    pub(crate) fn append_suffix_to_current_row(&mut self, text: &str) {
        match self.table.last_mut() {
            Some(Row::Regular(_, suffix)) => suffix.push_str(text),
            _ => panic!("State error: Not inside a regular table row."),
        }
    }

    pub(crate) fn start_new_line(&mut self) {
        self.table.push(Row::Regular(Vec::new(), String::new()))
    }

    pub(crate) fn render(&mut self, target: &mut super::Renderer) {
        // First, measure cols
        let max_number_of_columns = self
            .table
            .iter()
            .filter_map(|row| match row {
                Row::Regular(cols, _) => Some(cols.len()),
                _ => None,
            })
            .max();

        let mut max_widths_for_each_column = vec![0; max_number_of_columns.unwrap_or(0)];

        for row in &mut self.table {
            if let Row::Regular(row, _) = row {
                while row.last().map(|s| s.as_str()) == Some("") {
                    row.pop();
                }
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
