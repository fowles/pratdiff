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

  /// Display NUM lines of unchanged context before and after changes
  #[clap(short, long, value_name = "NUM", default_value_t = 3)]
  context: usize,
}

fn type_suffix(f: &Contents) -> &'static str {
  match f {
    Contents::Binary(_) => " (binary)",
    _ => "",
  }
}

fn print_diff(lhs: &str, rhs: &str) {
  let d = pratdiff::diff_lines(lhs, rhs);
  println!("{:?}", d);
}

fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();

  let lhs = files::read(&args.lhs)?;
  let rhs = files::read(&args.rhs)?;
  if lhs.as_bytes() == rhs.as_bytes() {
    return Ok(());
  }

  match (&lhs, &rhs) {
    (Contents::Text(l), Contents::Text(r)) => {
      println!("{} {}", "---".red(), args.lhs.display().bold().white());
      println!("{} {}", "+++".green(), args.rhs.display().bold().white());
      print_diff(l, r);
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
