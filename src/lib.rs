mod cluster;
mod diff;
mod files;
mod hunks;
pub mod parse_diff;
mod printer;
mod styles;
mod tokens;

use std::path::Path;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, clap::ValueEnum)]
pub enum IgnoreWhitespace {
  #[default]
  LengthChanges,
  All,
  No,
}

pub use cluster::ClusterEntry;
pub use cluster::DiffCluster;
pub use cluster::DiffSignature;
pub use diff::DiffItem;
pub use diff::diff;
pub use files::FilePairEvent;
pub use files::walk_file_pairs;
pub use printer::Printer;
pub use styles::Styles;
pub use tokens::tokenize_lines;

use crate::tokens::lines_to_bytes;

pub fn cluster_files(
  lhs: &Path,
  rhs: &Path,
  mode: IgnoreWhitespace,
) -> Vec<DiffCluster> {
  DiffCluster::cluster(walk_file_pairs(lhs, rhs), mode)
}

pub fn diff_files(
  p: &mut Printer,
  lhs: &Path,
  rhs: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
  for event in walk_file_pairs(lhs, rhs) {
    p.print_file_pair_event(event)?;
  }
  Ok(())
}

/// Re-render a piped diff with token-level colorization.
pub fn rediff(
  p: &mut Printer,
  input: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
  let diffs = parse_diff::parse(input);
  for file_diff in &diffs {
    p.print_parsed_file_diff(file_diff)?;
  }
  Ok(())
}

/// Cluster mutations extracted from a parsed diff by change signature.
pub fn rediff_cluster(
  diffs: &[parse_diff::ParsedFileDiff],
  mode: IgnoreWhitespace,
) -> Vec<DiffCluster> {
  let events = diffs.iter().flat_map(|fd| {
    fd.hunks.iter().flat_map(move |hunk| {
      hunk.items.iter().filter_map(move |item| {
        if let parse_diff::ParsedHunkItem::Mutation(m) = item {
          Some(FilePairEvent::TextDiff {
            lhs_path: fd.old_path.clone(),
            rhs_path: fd.new_path.clone(),
            lhs_content: lines_to_bytes(&m.old),
            rhs_content: lines_to_bytes(&m.new),
          })
        } else {
          None
        }
      })
    })
  });
  DiffCluster::cluster(events, mode)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rediff_cluster_groups_identical() {
    // Two file sections each with an identical foo->bar mutation.
    let input = b"\
--- a/x.txt\n\
+++ b/x.txt\n\
@@ -1 +1 @@\n\
-foo\n\
+bar\n\
--- a/y.txt\n\
+++ b/y.txt\n\
@@ -1 +1 @@\n\
-foo\n\
+bar\n\
";
    let diffs = parse_diff::parse(input);
    let clusters = rediff_cluster(&diffs, IgnoreWhitespace::No);
    assert_eq!(clusters.len(), 1);
    let total: usize = clusters[0].entries.values().sum();
    assert_eq!(total, 2);
  }

  #[test]
  fn rediff_cluster_separates_different() {
    let input = b"\
--- a/x.txt\n\
+++ b/x.txt\n\
@@ -1 +1 @@\n\
-foo\n\
+bar\n\
--- a/y.txt\n\
+++ b/y.txt\n\
@@ -1 +1 @@\n\
-hello\n\
+world\n\
";
    let diffs = parse_diff::parse(input);
    let clusters = rediff_cluster(&diffs, IgnoreWhitespace::No);
    assert_eq!(clusters.len(), 2);
  }
}
