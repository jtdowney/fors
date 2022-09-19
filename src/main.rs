use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use nix::sys;
use nix::unistd::Pid;
use output::Output;
use procfile_parser::Error;
use std::path::Path;
use std::process::{Command, Stdio};
use std::result;
use std::sync;
use std::sync::{Arc, Barrier};
use std::thread;

extern crate clap;
extern crate nix;
#[macro_use]
extern crate nom;
extern crate term;

mod output;
mod procfile_parser;

pub type Result<T> = result::Result<T, Error>;

fn check<P: AsRef<Path>>(procfile_path: P) -> Result<()> {
    match procfile_parser::load(procfile_path) {
        Ok(entries) => {
            let names: Vec<&str> = entries
                .iter()
                .map(|ref entry| entry.name.as_str())
                .collect();
            println!("Valid procfile ({})", names.join(", "));
        }
        Err(Error::Parsing) => println!("Invalid procfile"),
        Err(e) => return Err(e),
    }
    Ok(())
}

fn start(procfile_path: &Path, root_path: &Path) -> Result<()> {
    let processes = try!(procfile_parser::load(procfile_path));
    let padding = processes
        .iter()
        .map(|process| process.name.len())
        .max()
        .unwrap();

    let barrier = Arc::new(Barrier::new(processes.len() + 1));
    let (tx, rx) = sync::mpsc::channel();
    for (i, process) in processes.into_iter().enumerate() {
        let tx = tx.clone();
        let barrier = barrier.clone();
        let root_path = root_path.to_path_buf();
        thread::spawn(move || {
            let mut output = Output::new(padding);
            let mut child = Command::new("sh")
                .arg("-c")
                .arg(&process.command)
                .stdout(Stdio::piped())
                .current_dir(&root_path)
                .spawn()
                .unwrap();
            tx.send(child.id() as i32).unwrap();
            barrier.wait();

            loop {
                output.line_reader(&process.name, i, &mut child).unwrap();
            }
        });
    }

    barrier.wait();
    while let Ok(pid) = rx.try_recv() {
        let pid = Pid::from_raw(pid);
        sys::wait::waitpid(pid, None).unwrap();
    }

    Ok(())
}

fn extract_value<'a>(name: &str, args_list: &[&'a ArgMatches]) -> Option<&'a str> {
    for (i, args) in args_list.iter().enumerate() {
        if args.occurrences_of(name) > 0 || i == args_list.len() - 1 {
            return args.value_of(name);
        }
    }

    unreachable!()
}

fn main() {
    let app = App::new("fors")
        .version(env!("CARGO_PKG_VERSION"))
        .author("John Downey <jdowney@gmail.com>")
        .about("Run commands in a Procfile")
        .global_setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::VersionlessSubcommands)
        .arg(
            Arg::with_name("procfile")
                .short("f")
                .long("procfile")
                .takes_value(true)
                .value_name("PROCFILE")
                .default_value("Procfile")
                .global(true)
                .help("Set the file to use"),
        )
        .arg(
            Arg::with_name("root")
                .short("d")
                .long("root")
                .takes_value(true)
                .value_name("ROOT")
                .default_value(".")
                .global(true)
                .help("Set the directory to run in"),
        )
        .subcommand(SubCommand::with_name("check").about("Validate application Procfile"))
        .subcommand(SubCommand::with_name("start").about("Run application Procfile"));
    let args = app.get_matches();

    let result = match args.subcommand() {
        ("check", Some(subcommand_args)) => {
            let root = extract_value("root", &[subcommand_args, &args]).unwrap();
            let procfile = extract_value("procfile", &[subcommand_args, &args]).unwrap();
            let root_path = Path::new(root);
            let procfile_path = root_path.join(procfile);
            check(&procfile_path)
        }
        ("start", Some(subcommand_args)) => {
            let root = extract_value("root", &[subcommand_args, &args]).unwrap();
            let procfile = extract_value("procfile", &[subcommand_args, &args]).unwrap();
            let root_path = Path::new(root);
            let procfile_path = root_path.join(procfile);
            start(&procfile_path, &root_path)
        }
        ("", _) => {
            let root = args.value_of("root").unwrap();
            let procfile = args.value_of("procfile").unwrap();
            let root_path = Path::new(root);
            let procfile_path = root_path.join(procfile);
            start(&procfile_path, &root_path)
        }
        _ => unreachable!(),
    };
    result.unwrap();
}
