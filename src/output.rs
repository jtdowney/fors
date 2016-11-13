use std::io::{Read, Result};
use std::process::Child;
use term;

pub struct Output {
    padding: usize,
    terminal: Box<term::StdoutTerminal>,
}

const COLORS: [u16; 12] = [term::color::CYAN,
                           term::color::YELLOW,
                           term::color::GREEN,
                           term::color::MAGENTA,
                           term::color::RED,
                           term::color::BLUE,
                           term::color::BRIGHT_CYAN,
                           term::color::BRIGHT_YELLOW,
                           term::color::BRIGHT_GREEN,
                           term::color::BRIGHT_MAGENTA,
                           term::color::BRIGHT_RED,
                           term::color::BRIGHT_BLUE];

impl Output {
    pub fn new(padding: usize) -> Output {
        Output {
            padding: padding,
            terminal: term::stdout().unwrap(),
        }
    }

    pub fn line_reader(&mut self, name: &str, index: usize, child: &mut Child) -> Result<()> {
        let color = COLORS[index % COLORS.len()];
        let mut buffer = Vec::new();
        loop {
            let mut temp = [0; 1024];
            let read = match child.stdout {
                Some(ref mut r) => try!(r.read(&mut temp)),
                None => break,
            };

            if read == 0 {
                if !buffer.is_empty() {
                    try!(self.write_line(name, color, &buffer));
                }
                break;
            }

            let mut slice = &temp[..read];
            loop {
                let position = match slice.iter().position(|b| *b == b'\n') {
                    Some(v) => v,
                    None => break,
                };

                let before = &slice[..position + 1];
                buffer.extend_from_slice(&before);
                try!(self.write_line(name, color, &buffer));
                buffer.clear();

                slice = &slice[position + 1..];
            }

            buffer.extend_from_slice(slice);
            break;
        }

        Ok(())
    }

    fn write_line(&mut self, name: &str, color: u16, buffer: &[u8]) -> Result<()> {
        try!(self.terminal.fg(color));
        try!(write!(self.terminal, "{1:0$} | ", self.padding, name));
        try!(self.terminal.reset());
        try!(self.terminal.flush());
        try!(self.terminal.write(buffer));
        try!(self.terminal.flush());

        Ok(())
    }
}
