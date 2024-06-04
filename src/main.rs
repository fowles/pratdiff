use anstream::println;
use clap::Parser;
use owo_colors::OwoColorize;
use std::error::Error;

pub mod files;

#[derive(Parser, Debug)]
#[command(version)]
#[command(about = "Diff files using patience algorithm")]
struct Args {
  /// Path to old file or `-` for stdin.
  #[clap(name = "OLD_FILE")]
  lhs: String,

  /// Path to new file or `-` for stdin.
  #[clap(name = "NEW_FILE")]
  rhs: String,
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
