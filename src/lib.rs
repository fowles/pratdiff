mod cluster;
mod diff;
mod files;
mod hunks;
mod printer;
mod styles;
mod tokens;

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

use std::path::Path;

pub fn cluster_files(lhs: &Path, rhs: &Path) -> Vec<DiffCluster> {
  DiffCluster::cluster(walk_file_pairs(lhs, rhs))
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
