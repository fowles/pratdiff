#![allow(unused)] // TODO(kfm): remove this

use owo_colors::{OwoColorize, Style, Styled};
use pratdiff::DiffItem;
use pratdiff::DiffItem::*;
use pratdiff::Hunk;
use std::fmt::Display;
use std::io::Write;

struct Styles {
  header: Style,
  separator: Style,
  both: Style,
  old: Style,
  new: Style,
}

impl Styles {
  fn default() -> Styles {
    Styles {
      header: Style::new().bold().white(),
      separator: Style::new().cyan(),
      both: Style::new().default_color(),
      old: Style::new().red(),
      new: Style::new().green(),
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
    lhs: &[&str],
    rhs: &[&str],
    diffs: &[DiffItem],
  ) {
    for d in diffs {
      match *d {
        Mutation { lhs_pos, lhs_len, rhs_len: 0, .. } => {
          self.print_deletion(&lhs[lhs_pos..lhs_pos + lhs_len]);
        }
        Mutation { rhs_pos, rhs_len, lhs_len: 0, .. } => {
          self.print_insertion(&rhs[rhs_pos..rhs_pos + rhs_len]);
        }
        Mutation { lhs_pos, lhs_len, rhs_pos, rhs_len } => {
          self.print_mutation(
            &lhs[lhs_pos..lhs_pos + lhs_len],
            &rhs[rhs_pos..rhs_pos + rhs_len],
          );
        }
        Match { lhs: lhs_pos, len, .. } => {
          self.print_match(&lhs[lhs_pos..lhs_pos + len]);
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

  fn print_mutation(&mut self, lhs: &[&str], rhs: &[&str]) {
    // TODO(kfm): token level diffing
    self.print_deletion(&lhs);
    self.print_insertion(&rhs);
  }
}

fn binary_suffix(is_binary: bool) -> &'static str {
  if is_binary {
    " (binary)"
  } else {
    ""
  }
}
