use owo_colors::{OwoColorize, Style};
use pratdiff::{Hunk, DiffItem, Side, diff, tokenize_lines};
use pratdiff::DiffItem::*;
use std::fmt::Display;
use std::io::Result;
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

  pub fn print_binary_files_differ(
    &mut self,
    lhs: &dyn Display,
    rhs: &dyn Display,
  ) -> Result<()> {
    writeln!(
      self.writer,
      "Binary files {} and {} differ",
      lhs.style(self.styles.old),
      rhs.style(self.styles.new),
    )?;
    Ok(())
  }

  pub fn print_file_header(
    &mut self,
    lhs: &dyn Display,
    rhs: &dyn Display,
  ) -> Result<()> {
    writeln!(
      self.writer,
      "{} {}",
      "---".style(self.styles.old),
      lhs.style(self.styles.header),
    )?;
    writeln!(
      self.writer,
      "{} {}",
      "+++".style(self.styles.new),
      rhs.style(self.styles.header)
    )?;
    Ok(())
  }

  pub fn print_diff(&mut self, lhs_all: &str, rhs_all: &str) -> Result<()> {
    let lhs: Vec<_> = lhs_all.lines().collect();
    let rhs: Vec<_> = rhs_all.lines().collect();
    let diffs = diff(&lhs, &rhs);
    let hunks = Hunk::build(self.context, &diffs);

    for h in hunks {
      self.print_hunk_header(&h)?;
      self.print_hunk_body(&lhs, &rhs, &h.diffs)?;
    }
    Ok(())
  }

  fn print_hunk_header(&mut self, h: &Hunk) -> Result<()> {
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
    )?;
    Ok(())
  }

  fn print_hunk_body(
    &mut self,
    lhs_lines: &[&str],
    rhs_lines: &[&str],
    diffs: &[DiffItem],
  ) -> Result<()> {
    for d in diffs {
      match &d {
        Mutation { lhs, rhs } => {
          if rhs.is_empty() {
            self.print_lines(&lhs_lines[lhs.clone()], "-", self.styles.old)?;
          } else if lhs.is_empty() {
            self.print_lines(&rhs_lines[rhs.clone()], "+", self.styles.new)?;
          } else {
            self
              .print_mutation(&lhs_lines[lhs.clone()], &rhs_lines[rhs.clone()])?;
          }
        }
        Match { lhs, .. } => {
          self.print_lines(&lhs_lines[lhs.clone()], " ", self.styles.both)?;
        }
      }
    }
    Ok(())
  }

  fn print_lines(
    &mut self,
    lines: &[&str],
    prefix: &str,
    style: Style,
  ) -> Result<()> {
    for line in lines {
      writeln!(self.writer, "{}{}", prefix.style(style), line.style(style))?;
    }
    Ok(())
  }

  fn print_mutation(
    &mut self,
    lhs_lines: &[&str],
    rhs_lines: &[&str],
  ) -> Result<()> {
    let lhs_tokens = tokenize_lines(lhs_lines);
    let rhs_tokens = tokenize_lines(rhs_lines);
    let diffs = diff(&lhs_tokens, &rhs_tokens);
    self.print_mutation_side(
      &lhs_tokens,
      &diffs,
      "-",
      Side::Lhs,
      self.styles.old,
      self.styles.old_dim,
    )?;
    self.print_mutation_side(
      &rhs_tokens,
      &diffs,
      "+",
      Side::Rhs,
      self.styles.new,
      self.styles.new_dim,
    )?;
    Ok(())
  }

  fn print_mutation_side(
    &mut self,
    tokens: &[&str],
    diffs: &[DiffItem],
    prefix: &str,
    side: Side,
    mutation: Style,
    matching: Style,
  ) -> Result<()> {
    write!(self.writer, "{}", prefix.style(mutation))?;
    for d in diffs {
      let style = if matches!(d, Match { .. }) { matching } else { mutation };
      for &t in &tokens[d.side(side)] {
        write!(self.writer, "{}", t.style(style))?;
        if t == "\n" {
          write!(self.writer, "{}", prefix.style(mutation))?;
        }
      }
    }
    writeln!(self.writer)?;
    Ok(())
  }
}
