use std::error::Error;
use std::io::Read;
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
#[command(about = "Diff files with token level colorization")]
#[command(override_usage = r#"
    pratdiff [OPTIONS] <OLD_FILE> <NEW_FILE>
        Diff two files or directory trees.

    <diff tool> | pratdiff [OPTIONS]
        Rediff an existing diff, adding pratdiff goodness.

        Examples:
          git diff | pratdiff
          diff -u old.txt new.txt | pratdiff --cluster"#)]
struct Args {
  /// Path to old file, directory tree, or `-` for stdin.
  #[clap(name = "OLD_FILE")]
  lhs: Option<PathBuf>,

  /// Path to new file, directory tree, or `-` for stdin.
  #[clap(name = "NEW_FILE")]
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

  /// How to treat whitespace while diffing
  #[clap(long, value_enum, default_value_t = pratdiff::IgnoreWhitespace::LengthChanges)]
  ignore_whitespace: pratdiff::IgnoreWhitespace,

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

  let mut stdout = anstream::stdout();

  match (args.lhs, args.rhs) {
    (Some(lhs), Some(rhs)) => {
      let common_prefix = if args.verbose_paths {
        PathBuf::new()
      } else {
        common_path(&lhs, &rhs).unwrap_or_default()
      };
      let mut p = pratdiff::Printer::new(
        &mut stdout,
        args.context,
        common_prefix,
        args.ignore_whitespace,
      );
      if args.cluster {
        let clusters =
          pratdiff::cluster_files(&lhs, &rhs, args.ignore_whitespace);
        p.print_clusters(&clusters)?;
      } else {
        pratdiff::diff_files(&mut p, &lhs, &rhs)?;
      }
    }
    (Some(_), None) => {
      Args::command()
        .error(
          clap::error::ErrorKind::MissingRequiredArgument,
          "NEW_FILE is required when OLD_FILE is supplied",
        )
        .exit();
    }
    (None, _) => {
      // Re-diff mode: read diff from stdin and re-render.
      let mut input = Vec::new();
      std::io::stdin().read_to_end(&mut input)?;
      let input = anstream::adapter::strip_bytes(&input).into_vec();

      let mut p = pratdiff::Printer::new(
        &mut stdout,
        args.context,
        PathBuf::new(),
        args.ignore_whitespace,
      );
      if args.cluster {
        let diffs = pratdiff::parse_diff::parse(&input);
        let clusters = pratdiff::rediff_cluster(&diffs, args.ignore_whitespace);
        p.print_clusters(&clusters)?;
      } else {
        pratdiff::rediff(&mut p, &input)?;
      }
    }
  }
  Ok(())
}
