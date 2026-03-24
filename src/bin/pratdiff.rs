use std::error::Error;
use std::path::PathBuf;

use clap::ColorChoice;
use clap::CommandFactory;
use clap::Parser;
use clap_complete_command::Shell;
use common_path::common_path;

#[derive(Parser, Debug)]
#[command(version = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("VERGEN_GIT_DESCRIBE"),
    ", built ",
    env!("VERGEN_BUILD_DATE"),
    ")"
))]
#[command(about = "Diff files using patience algorithm")]
struct Args {
  /// Path to old file, directory tree, or `-` for stdin.
  #[clap(name = "OLD_FILE", required_unless_present = "shell")]
  lhs: Option<PathBuf>,

  /// Path to new file, directory tree, or `-` for stdin.
  #[clap(name = "NEW_FILE", required_unless_present = "shell")]
  rhs: Option<PathBuf>,

  /// Display NUM lines of unchanged context before and after changes
  #[clap(short, long, value_name = "NUM", default_value_t = 3)]
  context: usize,

  /// Print full paths instead of stripping a common prefix
  #[clap(short, long)]
  verbose_paths: bool,

  #[clap(long, default_value_t = ColorChoice::Auto)]
  color: ColorChoice,

  /// Group diffs into clusters by change signature
  #[clap(long)]
  cluster: bool,

  /// The shell to generate the completions for
  #[arg(long = "completions", value_name = "SHELL", value_enum)]
  shell: Option<Shell>,
}

fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();
  if let Some(shell) = args.shell {
    shell.generate(&mut Args::command(), &mut std::io::stdout());
    return Ok(());
  }

  match args.color {
    ColorChoice::Auto => anstream::ColorChoice::Auto,
    ColorChoice::Always => anstream::ColorChoice::Always,
    ColorChoice::Never => anstream::ColorChoice::Never,
  }
  .write_global();

  let lhs = args.lhs.unwrap();
  let rhs = args.rhs.unwrap();
  let common_prefix = if args.verbose_paths {
    PathBuf::new()
  } else {
    common_path(&lhs, &rhs).unwrap_or_default()
  };

  let mut stdout = anstream::stdout();
  let mut p =
    pratdiff::Printer::default(&mut stdout, args.context, common_prefix);

  if args.cluster {
    let clusters = pratdiff::cluster_files(&lhs, &rhs);
    p.print_clusters(&clusters)?;
  } else {
    pratdiff::diff_files(&mut p, &lhs, &rhs)?;
  }
  Ok(())
}
