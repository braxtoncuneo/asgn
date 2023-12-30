use core::fmt;

use crate::fail_info::{FailLog, FailInfo};
use colored::Colorize;
use itertools::Itertools;

pub struct Table {
    width: usize,
    rows: Vec<Box<[String]>>,
}

impl Table {
    pub fn new(width: usize) -> Self {
        Table {
            width,
            rows: Vec::new(),
        }
    }

    pub fn add_row(&mut self, row: impl Into<Box<[String]>>) -> Result<(), FailLog> {
        let row = row.into();

        if row.len() != self.width {
            return Err(FailInfo::Custom(
                "Internal Error: Attempted to add a row to a table with a different width".to_owned(),
                "Contact the instructor.".to_owned()
            ).into_log());
        }

        self.rows.push(row);
        Ok(())
    }

    fn as_table_row(row: &[String], col_widths: &[usize]) -> String {
        row.iter()
            .zip(col_widths)
            .map(|(text, width)| format!(" {text:width$} "))
            .join("|")
    }

    pub fn as_csv(&self) -> String {
        self.rows.iter()
            .map(|row| row.iter().join(","))
            .join("\n")
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let widths: Vec<usize> = (0..self.width)
            .map(|i| self.rows.iter()
                .map(|row| row[i].len())
                .max().unwrap_or_default()
            )
            .collect();

        for (i, row) in self.rows.iter().enumerate() {
            let text = Self::as_table_row(row, &widths);
            let styled = match i {
                0 => text.reversed().to_string(),
                i if i % 2 == 0 => text.on_bright_black().to_string(),
                _ => text
            };
            writeln!(f, "{styled}")?;
        }

        Ok(())
    }
}
