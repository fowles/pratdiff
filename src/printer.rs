use std::error::Error;
use std::io::Result;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use diff::DiffItem;
use diff::DiffItem::*;
use diff::Side;
use owo_colors::OwoColorize;
use owo_colors::Style;

use crate::cluster::DiffCluster;
use crate::diff;
use crate::files::FilePairEvent;
use crate::hunks::Hunk;
use crate::styles::Styles;
use crate::tokenize_lines;
use crate::tokens::split_lines;

pub struct Printer<'a> {
  styles: Styles,
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

  pub fn print_file_pair_event(&mut self, event: FilePairEvent) -> Result<()> {
    match event {
      FilePairEvent::TextDiff {
        lhs_path,
        rhs_path,
        lhs_content,
        rhs_content,
      } => {
        self.print_file_header(lhs_path.as_deref(), rhs_path.as_deref())?;
        self.print_diff(true, &lhs_content, &rhs_content)?;
      }
      FilePairEvent::Binary { lhs_path, rhs_path } => {
        self.print_binary_files_differ(
          lhs_path.as_deref(),
          rhs_path.as_deref(),
        )?;
      }
      FilePairEvent::TypeMismatch { lhs_path, rhs_path } => {
        self.print_directory_mismatch(&lhs_path, &rhs_path)?;
      }
      FilePairEvent::IoError { lhs_path, rhs_path, err } => {
        writeln!(
          self.writer,
          "Error diffing {} and {}:\n{}",
          self
            .display_name(lhs_path.as_deref())
            .style(self.styles.old),
          self
            .display_name(rhs_path.as_deref())
            .style(self.styles.new),
          err,
        )?;
      }
    }
    Ok(())
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
      if p.metadata().unwrap().is_dir() { "directory" } else { "file" }
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

  pub fn print_diff(
    &mut self,
    include_headers: bool,
    lhs_all: &[u8],
    rhs_all: &[u8],
  ) -> Result<()> {
    let lhs = split_lines(lhs_all);
    let rhs = split_lines(rhs_all);
    let diffs = diff(&lhs, &rhs);
    let hunks = Hunk::build(self.context, &diffs);

    for h in hunks {
      if include_headers {
        self.print_hunk_header(&h)?;
      }
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
        "@@ -{},{} +{},{} @@",
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
    lhs_lines: &[&[u8]],
    rhs_lines: &[&[u8]],
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
    lines: &[&[u8]],
    prefix: &str,
    style: Style,
  ) -> Result<()> {
    for line in lines {
      let s = String::from_utf8_lossy(line);
      writeln!(self.writer, "{}{}", prefix.style(style), s.style(style))?;
    }
    Ok(())
  }

  fn print_mutation(
    &mut self,
    lhs_lines: &[&[u8]],
    rhs_lines: &[&[u8]],
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
    tokens: &[&[u8]],
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
        let s = String::from_utf8_lossy(t);
        write!(self.writer, "{}", s.style(style))?;
        if t == b"\n" {
          write!(self.writer, "{}", prefix.style(mutation))?;
        }
      }
    }
    writeln!(self.writer)?;
    Ok(())
  }

  pub fn print_clusters(&mut self, clusters: &[DiffCluster]) -> Result<()> {
    for cluster in clusters {
      self.print_cluster(cluster)?;
    }
    Ok(())
  }

  fn print_cluster(&mut self, cluster: &DiffCluster) -> Result<()> {
    let total: usize = cluster.entries.values().sum();
    let entry_count =
      |n| format!("{} {}", n, if 1 == n { "entry" } else { "entries" });
    writeln!(
      self.writer,
      "{}",
      format!("=== cluster contains {}", entry_count(total))
        .style(self.styles.header),
    )?;
    for (entry, &count) in &cluster.entries {
      let lhs = self.display_name(entry.lhs_path.as_deref());
      let rhs = self.display_name(entry.rhs_path.as_deref());
      write!(self.writer, "{}", "= ".style(self.styles.separator))?;
      write!(self.writer, "{}", lhs.style(self.styles.old))?;
      write!(self.writer, "{}", " => ".style(self.styles.separator))?;
      write!(self.writer, "{}", rhs.style(self.styles.new))?;
      writeln!(
        self.writer,
        "{}",
        format!(": {}", entry_count(count)).style(self.styles.separator)
      )?;
    }
    writeln!(
      self.writer,
      "{}",
      "=== example diff: ".style(self.styles.separator)
    )?;
    self.print_diff(false, &cluster.exemplar_lhs, &cluster.exemplar_rhs)?;
    Ok(())
  }
}
