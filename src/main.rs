pub mod files;

use anstream::println;
use clap::Parser;
use files::Contents;
use owo_colors::OwoColorize;
use std::error::Error;
use std::path::PathBuf;

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

fn type_suffix(f: &Contents) -> &'static str {
  match f {
    Contents::Binary(_) => " (binary)",
    _ => "",
  }
}

fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();

  let lhs = files::read(&args.lhs)?;
  let rhs = files::read(&args.rhs)?;
  if lhs.as_bytes() == rhs.as_bytes() {
    return Ok(());
  }

  match (&lhs, &rhs) {
    (Contents::Text(l), Contents::Text(s)) => {
      println!("{}", "text".blue());
    }
    _ => {
      println!(
        "Files {}{} and {}{} differ",
        args.lhs.display().red(),
        type_suffix(&lhs),
        args.rhs.display().green(),
        type_suffix(&rhs),
      );
    }
  }

  Ok(())
}
