use clap::Parser;
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

mod files;
mod printer;

use files::Contents::{Binary, Text};

fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();

  let lhs = files::read(&args.lhs)?;
  let rhs = files::read(&args.rhs)?;
  if lhs.as_bytes() == rhs.as_bytes() {
    return Ok(());
  }

  let mut p =
    printer::Printer::default(Box::new(anstream::stdout()), args.context);

  if let (Text(l), Text(r)) = (&lhs, &rhs) {
    p.print_file_header(&args.lhs.display(), &args.rhs.display());
    p.print_diff(l, r);
  } else {
    p.print_files_differ(
      &args.lhs.display(),
      matches!(lhs, Binary(_)),
      &args.rhs.display(),
      matches!(rhs, Binary(_)),
    );
  }

  Ok(())
}
