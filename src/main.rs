mod files;
mod printer;

use clap::{ColorChoice, Parser};
use common_path::common_path;
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

  /// Print full paths instead of stripping a common prefix
  #[clap(short, long)]
  verbose_paths: bool,

  #[clap(long, default_value_t = ColorChoice::Auto)]
  color: ColorChoice,
}

fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();
  match args.color {
    ColorChoice::Auto => anstream::ColorChoice::Auto,
    ColorChoice::Always => anstream::ColorChoice::Always,
    ColorChoice::Never => anstream::ColorChoice::Never,
  }
  .write_global();

  let common_prefix = if args.verbose_paths {
    PathBuf::new()
  } else {
    common_path(&args.lhs, &args.rhs).unwrap_or_default()
  };

  let mut p = printer::Printer::default(
    Box::new(anstream::stdout()),
    args.context,
    common_prefix,
  );
  files::diff(&mut p, &args.lhs, &args.rhs)
}
