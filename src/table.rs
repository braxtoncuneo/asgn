
use crate::fail_info:: {
    FailLog,
    FailInfo,
};

use colored::Colorize;



pub struct Table {
    width  : usize,
    rows : Vec<Vec<String>>,
}

impl Table {

    pub fn new(width : usize) -> Self {
        Table {
            width,
            rows: Vec::new(),
        }
    }

    pub fn add_row(&mut self,row : Vec<String>) -> Result<(),FailLog> {
        if row.len() != self.width {
            return Err(FailInfo::Custom(
                "Internal Error: Attempted to add a row to a table with a different width".to_string(),
                "Contact the instructor.".to_string()
            ).into_log());
        }
        self.rows.push(row);
        Ok(())
    }

    fn as_table_row(row : &Vec<String>, col_widths : &Vec<usize>) -> String {
        row .iter()
            .zip(col_widths.iter())
            .enumerate()
            .map(|(idx,(text,width))| {
                if idx == 0 {
                    format!(" {:width$} ",text,width = width)
                } else {
                    format!("| {:width$} ",text,width = width)
                }
            }).fold(String::new(),|acc,val| acc + &val)
    }

    pub fn as_table(&self) -> String {
        let zeros : Vec<usize> = std::iter::repeat(0).take(self.width).collect();

        let widths : Vec<usize> = self.rows.iter()
            .map(|row| row.iter().map(|text| text.len()))
            .fold(zeros,|acc,row| -> Vec<usize> {
                acc.iter().zip(row).map( |(x,y)| std::cmp::max(*x,y)).collect()
            });

        self.rows.iter()
            .map(|row| Self::as_table_row(row,&widths))
            .enumerate()
            .map(|(idx,text)| {
                if idx == 0 {
                    text.reversed().to_string()
                } else if (idx % 2) == 0 {
                    text.on_bright_black().to_string()
                } else {
                    text
                }
            }).fold(String::new(),|acc,val| acc + &val + "\n" )
    }

    pub fn as_csv(&self) -> String {

        self.rows.iter()
            .map(|row|
                row .iter()
                    .map(|s| s.to_string() + ",")
                    .fold(String::new(), |acc,val| acc + &val )
            ).map(|s| s + "\n")
            .fold(String::new(), |acc,val| acc + &val)

    }

}


