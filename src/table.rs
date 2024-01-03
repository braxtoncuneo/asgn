use core::fmt;

use crate::{error, util::color::{STYLE_RESET, COLOR_REVERSED, BG_LIGHT_BLACK}};
use itertools::Itertools;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Error;

impl From<Error> for error::Error {
    fn from(_: Error) -> Self {
        error::Error::table_error()
    }
}

pub struct Table {
    width: usize,
    rows: Vec<Box<[String]>>,
}

impl Table {
    pub const NONE_REPR: &'static str = "NONE";

    pub fn new(header: impl Into<Box<[String]>>) -> Self {
        let header = header.into();
        Table {
            width: header.len(),
            rows: vec![header],
        }
    }

    pub fn extend<Row: Into<Box<[String]>>, I: IntoIterator<Item=Row>>(&mut self, rows: I) -> Result<(), Error> {
        let rows = rows.into_iter();
        self.rows.reserve(rows.size_hint().0);

        rows.map(Row::into).try_for_each(|row|
            if row.len() == self.width {
                self.rows.push(row);
                Ok(())
            } else {
                Err(Error)
            }
        )
    }

    pub fn as_csv(&self) -> String {
        self.rows.iter()
            .map(|row| row.iter().join(","))
            .join("\n")
    }

    pub fn option_repr(value: Option<impl fmt::Display>) -> String {
        value.as_ref().map(ToString::to_string).unwrap_or_else(|| Self::NONE_REPR.to_owned())
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
            let text = row.iter()
                .zip(&widths)
                .map(|(text, width)| format!(" {text:width$} "))
                .join("|");

            match i {
                0 => writeln!(f, "{COLOR_REVERSED}{text}{STYLE_RESET}"),
                i if i % 2 == 0 => writeln!(f, "{BG_LIGHT_BLACK}{text}{STYLE_RESET}"),
                _ => writeln!(f, "{text}"),
            }?
        }

        Ok(())
    }
}
