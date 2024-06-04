use anstream::println;
use clap::Parser;
use owo_colors::OwoColorize;
use std::error::Error;
use std::path::PathBuf;

pub mod files;

#[derive(Parser, Debug)]
#[command(version)]
#[command(about = "Diff files using patience algorithm")]
struct Args {
  /// Path to old file or `-` for stdin.
  #[clap(name = "OLD_FILE")]
  lhs: PathBuf,

  /// Path to new file or `-` for stdin.
  #[clap(name = "NEW_FILE")]
  rhs: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();

  let lhs = files::read(&args.lhs)?;
  let rhs = files::read(&args.rhs)?;

  match (&lhs, &rhs) {
    (files::Contents::Text(_), files::Contents::Text(_)) => {
        println!("{}", "text".blue());
    },
    _ => {
      if lhs.as_bytes() == rhs.as_bytes() {
        println!("{}", "match".green());
      } else {
        println!("{}", "diff".green());
      }
    }
  }

  Ok(())
}
