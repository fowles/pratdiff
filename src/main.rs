use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

mod files;
mod printer;

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

fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();
  let mut p =
    printer::Printer::default(Box::new(anstream::stdout()), args.context);
  files::diff_files(&mut p, &args.lhs, &args.rhs)
}
