use crate::style::Styles;
use diff::DiffItem::*;
use diff::DiffItem;
use diff::Hunk;
use diff::Side;
use crate::{diff, tokenize_lines};
use owo_colors::OwoColorize;
use owo_colors::Style;
use std::error::Error;
use std::io::Result;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Printer<'a> {
  pub styles: Styles,
  writer: &'a mut dyn Write,
  context: usize,
  common_prefix: PathBuf,
}

impl<'a> Printer<'a> {
  pub fn default(
    writer: &'a mut dyn Write,
    context: usize,
    common_prefix: PathBuf,
  ) -> Printer<'a> {
    Printer {
      styles: Styles::simple(),
      writer,
      context,
      common_prefix,
    }
  }

  fn display_name(&self, p: Option<&Path>) -> String {
    let Some(p) = p else {
      return "/dev/null".into();
    };
    let stripped = p.strip_prefix(&self.common_prefix).unwrap_or(p);
    if let Ok(link) = std::fs::read_link(p) {
      let stripped_link =
        link.strip_prefix(&self.common_prefix).unwrap_or(&link);
      return format!("{} -> {}", stripped.display(), stripped_link.display());
    }
    stripped.display().to_string()
  }

  pub fn print_error(
    &mut self,
    lhs: Option<&Path>,
    rhs: Option<&Path>,
    err: Box<dyn Error>,
  ) -> Result<()> {
    writeln!(
      self.writer,
      "Error diffing {} and {}:\n{}",
      self.display_name(lhs).style(self.styles.old),
      self.display_name(rhs).style(self.styles.new),
      err
    )
  }

  pub fn print_directory_mismatch(
    &mut self,
    lhs: &Path,
    rhs: &Path,
  ) -> Result<()> {
    fn ft(p: &Path) -> &str {
      if p.metadata().unwrap().is_dir() {
        "directory"
      } else {
        "file"
      }
    }
    writeln!(
      self.writer,
      "File/directory mistmatch:\n  {} is a {}\n  {} is a {}",
      self.display_name(Some(lhs)).style(self.styles.old),
      ft(lhs),
      self.display_name(Some(rhs)).style(self.styles.new),
      ft(rhs),
    )
  }

  pub fn print_binary_files_differ(
    &mut self,
    lhs: Option<&Path>,
    rhs: Option<&Path>,
  ) -> Result<()> {
    writeln!(
      self.writer,
      "Binary files {} and {} differ",
      self.display_name(lhs).style(self.styles.old),
      self.display_name(rhs).style(self.styles.new),
    )?;
    Ok(())
  }

  pub fn print_file_header(
    &mut self,
    lhs: Option<&Path>,
    rhs: Option<&Path>,
  ) -> Result<()> {
    writeln!(
      self.writer,
      "{} {}",
      "---".style(self.styles.old),
      self.display_name(lhs).style(self.styles.header),
    )?;
    writeln!(
      self.writer,
      "{} {}",
      "+++".style(self.styles.new),
      self.display_name(rhs).style(self.styles.header)
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
    let (l, r) = (h.lhs(), h.rhs());
    writeln!(
      self.writer,
      "{}",
      format!(
        "@@ -{},{} +{},{}  @@",
        l.start + 1,
        l.len(),
        r.start + 1,
        r.len()
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
            self.print_mutation(
              &lhs_lines[lhs.clone()],
              &rhs_lines[rhs.clone()],
            )?;
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
