use nom::{IResult, line_ending, not_line_ending, space};
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;
use std::str;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Parsing,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Io(error)
    }
}

#[derive(Debug, PartialEq)]
pub struct ProcfileEntry {
    pub name: String,
    pub command: String,
}

fn valid_process_name(b: u8) -> bool {
    let c = b as char;
    c.is_alphanumeric() || c == '_'
}

named!(process_name, take_while!(valid_process_name));
named!(process_entry<&[u8], ProcfileEntry>,
       chain!(
           name: map_res!(process_name, str::from_utf8) ~
           char!(':') ~
           opt!(space) ~
           command: map_res!(not_line_ending, str::from_utf8) ~
           alt_complete!(eof!() | line_ending),
           || {
               ProcfileEntry {
                   name: name.to_string(),
                   command: command.to_string(),
               }
           }
       )
);
named!(pub process_entries<&[u8], Vec<ProcfileEntry> >, many1!(process_entry));

pub fn load<P: AsRef<Path>>(procfile_path: P) -> Result<Vec<ProcfileEntry>, Error> {
    let mut buffer = Vec::new();
    let mut file = try!(File::open(procfile_path));
    try!(file.read_to_end(&mut buffer));

    match process_entries(&buffer) {
        IResult::Done(ref input, _) if !input.is_empty() => Err(Error::Parsing),
        IResult::Done(_, result) => Ok(result),
        _ => Err(Error::Parsing),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let input = b"test: command";
        let result = process_entries(input.as_ref());
        assert_eq!(result.unwrap().1,
                   vec![ProcfileEntry {
                            name: "test".to_string(),
                            command: "command".to_string(),
                        }]);
    }

    #[test]
    fn test_multiple_commands() {
        let input = b"test: command\nhello: world\n";
        let result = process_entries(input.as_ref());
        println!("{:?}", result);
        assert_eq!(result.unwrap().1,
                   vec![ProcfileEntry {
                            name: "test".to_string(),
                            command: "command".to_string(),
                        },
                        ProcfileEntry {
                            name: "hello".to_string(),
                            command: "world".to_string(),
                        }]);
    }
}
