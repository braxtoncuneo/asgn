use std::fmt;

use termion::terminal_size;

pub mod color {
    use termion::{color::*, style};

    pub const FG_RED: Fg<Red> = Fg(Red);
    pub const FG_GREEN: Fg<Green> = Fg(Green);
    pub const FG_YELLOW: Fg<Yellow> = Fg(Yellow);

    pub const BG_LIGHT_BLACK: Bg<LightBlack> = Bg(LightBlack);

    pub const TEXT_BOLD: style::Bold = style::Bold;
    pub const COLOR_REVERSED: style::Invert = style::Invert;
    pub const STYLE_RESET: style::Reset = style::Reset;
}

use self::color::*;

pub enum Hline { Normal, Bold }

impl fmt::Display for Hline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match terminal_size() {
            Ok((width, _)) => match self {
                Self::Normal => write!(f, "{:-<1$}", "", width as usize),
                Self::Bold => write!(f, "{TEXT_BOLD}{:=<1$}{STYLE_RESET}", "", width as usize),
            }
            Err(_) => Ok(()),
        }
    }
}
