#![allow(unused)] // TODO(kfm): remove this

use anstream::println;
use clap::Parser;
use owo_colors::{OwoColorize, Style, Styled};
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

fn type_suffix(f: &files::Contents) -> &'static str {
  match f {
    files::Contents::Binary(_) => " (binary)",
    _ => "",
  }
}

fn offset_range(lhs: (usize, usize), rhs: (usize, usize)) -> String {
  format!("@@ -{},{} +{},{} @@", lhs.0 + 1, lhs.1, rhs.0 + 1, rhs.1)
}

use pratdiff::DiffItem;
use pratdiff::DiffItem::*;

fn lines(
  lhs: &[&str],
  rhs: &[&str],
  context: usize,
  diff: &DiffItem,
) -> Vec<(String, Style)> {
  let header: Style = Style::new().cyan();
  let both: Style = Style::new().default_color();
  let old: Style = Style::new().red();
  let new: Style = Style::new().green();

  match *diff {
    Mutation { lhs_pos, lhs_len, rhs_pos, rhs_len } => {
      let mut r = Vec::new();
      for line in &lhs[lhs_pos..lhs_pos + lhs_len] {
        r.push((format!("-{}", line), old));
      }
      for line in &rhs[rhs_pos..rhs_pos + rhs_len] {
        r.push((format!("+{}", line), new));
      }
      r
    }
    Match { lhs: lhs_pos, rhs: rhs_pos, len } => {
      let mut r = Vec::new();
      if len > 2 * context {
        for line in &lhs[lhs_pos..lhs_pos + context] {
          r.push((format!(" {}", line), both));
        }
        r.push((
          format!(
            "@@ -{},{} +{},{}  @@",
            lhs_pos + context,
            0,
            rhs_pos + context,
            0
          ),
          header,
        ));
        for line in &lhs[lhs_pos + len - context..lhs_pos + len] {
          r.push((format!(" {}", line), both));
        }
      } else {
        for line in &lhs[lhs_pos..lhs_pos + len] {
          r.push((format!(" {}", line), both));
        }
      }
      r
    }
  }
}

fn print_diff(context: usize, lhs_all: &str, rhs_all: &str) {
  let header: Style = Style::new().cyan();
  let old: Style = Style::new().red();
  let new: Style = Style::new().green();

  let lhs: Vec<_> = lhs_all.lines().collect();
  let rhs: Vec<_> = rhs_all.lines().collect();
  let diffs = pratdiff::diff(&lhs, &rhs);

  let mut lhs_printed: usize = 0;
  let mut rhs_printed: usize = 0;
  for diff in diffs {
    for (line, style) in lines(&lhs, &rhs, context, &diff) {
      println!("{}", line.style(style));
    }
  }
}

fn main() -> Result<(), Box<dyn Error>> {
  let header: Style = Style::new().bold().white();
  let old: Style = Style::new().red();
  let new: Style = Style::new().green();

  let args = Args::parse();

  let lhs = files::read(&args.lhs)?;
  let rhs = files::read(&args.rhs)?;
  if lhs.as_bytes() == rhs.as_bytes() {
    return Ok(());
  }

  match (&lhs, &rhs) {
    (files::Contents::Text(l), files::Contents::Text(r)) => {
      println!("{} {}", "---".style(old), args.lhs.display().style(header));
      println!("{} {}", "+++".style(new), args.rhs.display().style(header));
      print_diff(args.context, l, r);
    }
    _ => {
      println!(
        "Files {}{} and {}{} differ",
        args.lhs.display().style(old),
        type_suffix(&lhs),
        args.rhs.display().style(new),
        type_suffix(&rhs),
      );
    }
  }

  Ok(())
}
