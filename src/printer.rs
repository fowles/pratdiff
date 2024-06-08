#![allow(unused)] // TODO(kfm): remove this

use owo_colors::{OwoColorize, Style, Styled};
use pratdiff::DiffItem;
use pratdiff::DiffItem::*;
use pratdiff::Hunk;
use regex::Regex;
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
            self.print_deletion(&lhs_lines[lhs.clone()]);
          } else if (lhs.is_empty()) {
            self.print_insertion(&rhs_lines[rhs.clone()]);
          } else {
            self
              .print_mutation(&lhs_lines[lhs.clone()], &rhs_lines[rhs.clone()]);
          }
        }
        Match { lhs, .. } => {
          self.print_match(&lhs_lines[lhs.clone()]);
        }
      }
    }
  }

  fn print_match(&mut self, lines: &[&str]) {
    for line in lines {
      writeln!(self.writer, " {}", line.style(self.styles.both));
    }
  }

  fn print_deletion(&mut self, lines: &[&str]) {
    for line in lines {
      writeln!(self.writer, "{}", format!("-{}", line).style(self.styles.old));
    }
  }

  fn print_insertion(&mut self, lines: &[&str]) {
    for line in lines {
      writeln!(self.writer, "{}", format!("+{}", line).style(self.styles.new));
    }
  }

  fn print_mutation(&mut self, lhs_lines: &[&str], rhs_lines: &[&str]) {
    let re = Regex::new(r"\b").unwrap();
    let tokenize = |&line| re.split(line).chain(std::iter::once("\n"));

    let mut lhs_tokens: Vec<_> = lhs_lines.iter().flat_map(tokenize).collect();
    lhs_tokens.pop();

    let mut rhs_tokens: Vec<_> = rhs_lines.iter().flat_map(tokenize).collect();
    rhs_tokens.pop();

    let diffs = pratdiff::diff(&lhs_tokens, &rhs_tokens);

    write!(self.writer, "{}", "-".style(self.styles.old));
    for d in &diffs {
      match &d {
        Match { lhs, .. } => {
          for &t in &lhs_tokens[lhs.clone()] {
            write!(self.writer, "{}", t.style(self.styles.old_dim));
            if t == "\n" {
              write!(self.writer, "{}", "-".style(self.styles.old));
            }
          }
        }
        Mutation { lhs, .. } => {
          for &t in &lhs_tokens[lhs.clone()] {
            write!(self.writer, "{}", t.style(self.styles.old));
            if t == "\n" {
              write!(self.writer, "{}", "-".style(self.styles.old));
            }
          }
        }
      }
    }
    writeln!(self.writer);

    write!(self.writer, "{}", "+".style(self.styles.new));
    for d in &diffs {
      match &d {
        Match { rhs, .. } => {
          for &t in &rhs_tokens[rhs.clone()] {
            write!(self.writer, "{}", t.style(self.styles.new_dim));
            if t == "\n" {
              write!(self.writer, "{}", "+".style(self.styles.new));
            }
          }
        }
        Mutation { rhs, .. } => {
          for &t in &rhs_tokens[rhs.clone()] {
            write!(self.writer, "{}", t.style(self.styles.new));
            if t == "\n" {
              write!(self.writer, "{}", "+".style(self.styles.new));
            }
          }
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
