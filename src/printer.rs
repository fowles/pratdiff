#![allow(unused)] // TODO(kfm): remove this

use anstream::println;
use owo_colors::{OwoColorize, Style, Styled};
use pratdiff::DiffItem;
use pratdiff::DiffItem::*;
use std::fmt::Display;
use std::io::Write;

pub struct Printer {
  header: Style,
  separator: Style,
  both: Style,
  old: Style,
  new: Style,
  writer: Box<dyn Write>,
  context: usize,
}

impl Printer {
  pub fn default(writer: Box<dyn Write>, context: usize) -> Printer {
    Printer {
      header: Style::new().bold().white(),
      separator: Style::new().cyan(),
      both: Style::new().default_color(),
      old: Style::new().red(),
      new: Style::new().green(),
      writer,
      context,
    }
  }

  pub fn print_files_differ(
    &mut self,
    lhs: &dyn Display,
    lhs_is_binary: bool,
    rhs: &dyn Display,
    rhs_is_binary: bool,
  ) {
    println!(
      "Files {}{} and {}{} differ",
      lhs.style(self.old),
      binary_suffix(lhs_is_binary),
      rhs.style(self.new),
      binary_suffix(rhs_is_binary),
    );
  }

  pub fn print_file_header(&mut self, lhs: &dyn Display, rhs: &dyn Display) {
    writeln!(
      self.writer,
      "{} {}",
      "---".style(self.old),
      lhs.style(self.header)
    );
    writeln!(
      self.writer,
      "{} {}",
      "+++".style(self.new),
      rhs.style(self.header)
    );
  }

  pub fn print_diff(&mut self, lhs_all: &str, rhs_all: &str) {
    let lhs: Vec<_> = lhs_all.lines().collect();
    let rhs: Vec<_> = rhs_all.lines().collect();
    let diffs = pratdiff::diff(&lhs, &rhs);

    for diff in diffs {
      self.print_lines(&lhs, &rhs, &diff);
    }
  }

  fn print_lines(&mut self, lhs: &[&str], rhs: &[&str], diff: &DiffItem) {
    match *diff {
      Mutation { lhs_pos, lhs_len, rhs_pos, rhs_len } => {
        for line in &lhs[lhs_pos..lhs_pos + lhs_len] {
          writeln!(self.writer, "{}", format!("-{}", line).style(self.old));
        }
        for line in &rhs[rhs_pos..rhs_pos + rhs_len] {
          writeln!(self.writer, "{}", format!("+{}", line).style(self.new));
        }
      }
      Match { lhs: lhs_pos, rhs: rhs_pos, len } => {
        if len <= 2 * self.context {
          for line in &lhs[lhs_pos..lhs_pos + len] {
            writeln!(self.writer, " {}", line.style(self.both));
          }
        } else {
          for line in &lhs[lhs_pos..lhs_pos + self.context] {
            writeln!(self.writer, " {}", line.style(self.both));
          }
          writeln!(
            self.writer,
            "{}",
            format!(
              "@@ -{},{} +{},{}  @@",
              lhs_pos + self.context,
              0,
              rhs_pos + self.context,
              0
            )
            .style(self.separator)
          );
          for line in &lhs[lhs_pos + len - self.context..lhs_pos + len] {
            writeln!(self.writer, " {}", line.style(self.both));
          }
        }
      }
    }
  }
}

fn binary_suffix(is_binary: bool) -> &'static str {
  if is_binary {
    " (binary)"
  } else {
    ""
  }
}
