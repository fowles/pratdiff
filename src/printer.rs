#![allow(unused)] // TODO(kfm): remove this

use owo_colors::{OwoColorize, Style, Styled};
use pratdiff::DiffItem::*;
use pratdiff::Hunk;
use pratdiff::{DiffItem, Side};
use std::fmt::Display;
use std::io::Write;

struct Styles {
  header: Style,
  separator: Style,
  both: Style,
  old: Style,
  old_dim: Style,
  new: Style,
  new_dim: Style,
}

impl Styles {
  fn default() -> Styles {
    Styles {
      header: Style::new().bold().white(),
      separator: Style::new().cyan(),
      both: Style::new().default_color(),
      old: Style::new().red(),
      new: Style::new().green(),
      old_dim: Style::new().dimmed(),
      new_dim: Style::new().default_color(),
    }
  }
}

pub struct Printer {
  styles: Styles,
  writer: Box<dyn Write>,
  context: usize,
}

impl Printer {
  pub fn default(writer: Box<dyn Write>, context: usize) -> Printer {
    Printer {
      styles: Styles::default(),
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
    writeln!(
      self.writer,
      "Files {}{} and {}{} differ",
      lhs.style(self.styles.old),
      binary_suffix(lhs_is_binary),
      rhs.style(self.styles.new),
      binary_suffix(rhs_is_binary),
    );
  }

  pub fn print_file_header(&mut self, lhs: &dyn Display, rhs: &dyn Display) {
    writeln!(
      self.writer,
      "{} {}",
      "---".style(self.styles.old),
      lhs.style(self.styles.header),
    );
    writeln!(
      self.writer,
      "{} {}",
      "+++".style(self.styles.new),
      rhs.style(self.styles.header)
    );
  }

  pub fn print_diff(&mut self, lhs_all: &str, rhs_all: &str) {
    let lhs: Vec<_> = lhs_all.lines().collect();
    let rhs: Vec<_> = rhs_all.lines().collect();
    let diffs = pratdiff::diff(&lhs, &rhs);
    let hunks = Hunk::build(self.context, &diffs);

    for h in hunks {
      self.print_hunk_header(&h);
      self.print_hunk_body(&lhs, &rhs, &h.diffs);
    }
  }

  fn print_hunk_header(&mut self, h: &Hunk) {
    writeln!(
      self.writer,
      "{}",
      format!(
        "@@ -{},{} +{},{}  @@",
        h.lhs_pos() + 1,
        h.lhs_len(),
        h.rhs_pos() + 1,
        h.rhs_len()
      )
      .style(self.styles.separator)
    );
  }

  fn print_hunk_body(
    &mut self,
    lhs_lines: &[&str],
    rhs_lines: &[&str],
    diffs: &[DiffItem],
  ) {
    for d in diffs {
      match &d {
        Mutation { lhs, rhs } => {
          if (rhs.is_empty()) {
            self.print_lines(&lhs_lines[lhs.clone()], "-", self.styles.old);
          } else if (lhs.is_empty()) {
            self.print_lines(&rhs_lines[rhs.clone()], "+", self.styles.new);
          } else {
            self
              .print_mutation(&lhs_lines[lhs.clone()], &rhs_lines[rhs.clone()]);
          }
        }
        Match { lhs, .. } => {
          self.print_lines(&lhs_lines[lhs.clone()], " ", self.styles.both);
        }
      }
    }
  }

  fn print_lines(&mut self, lines: &[&str], prefix: &str, style: Style) {
    for line in lines {
      writeln!(self.writer, "{}{}", prefix.style(style), line.style(style));
    }
  }

  fn print_mutation(&mut self, lhs_lines: &[&str], rhs_lines: &[&str]) {
    let mut lhs_tokens = pratdiff::tokenize_lines(&lhs_lines);
    let mut rhs_tokens = pratdiff::tokenize_lines(&rhs_lines);
    let diffs = pratdiff::diff(&lhs_tokens, &rhs_tokens);
    self.print_mutation_side(
      &lhs_tokens,
      &diffs,
      "-",
      Side::Lhs,
      self.styles.old,
      self.styles.old_dim,
    );
    self.print_mutation_side(
      &rhs_tokens,
      &diffs,
      "+",
      Side::Rhs,
      self.styles.new,
      self.styles.new_dim,
    );
  }

  fn print_mutation_side(
    &mut self,
    tokens: &[&str],
    diffs: &[DiffItem],
    prefix: &str,
    side: Side,
    mutation: Style,
    matching: Style,
  ) {
    write!(self.writer, "{}", prefix.style(mutation));
    for d in diffs {
      let style = if matches!(d, Match { .. }) { matching } else { mutation };
      for &t in &tokens[d.side(side)] {
        write!(self.writer, "{}", t.style(style));
        if t == "\n" {
          write!(self.writer, "{}", prefix.style(mutation));
        }
      }
    }
    writeln!(self.writer);
  }
}

fn binary_suffix(is_binary: bool) -> &'static str {
  if is_binary {
    " (binary)"
  } else {
    ""
  }
}
