use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;

use crate::IgnoreWhitespace;
use crate::diff::DiffItem;
use crate::diff::diff;
use crate::files::FilePairEvent;
use crate::tokens::NormalizedBytes;
use crate::tokens::lines_to_bytes;
use crate::tokens::split_lines;
use crate::tokens::tokenize_lines;

/// A content-based signature for a single mutation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DiffSignature {
  lhs: u64,
  rhs: u64,
}

impl DiffSignature {
  pub fn new(
    lhs_lines: &[&[u8]],
    rhs_lines: &[&[u8]],
    mode: IgnoreWhitespace,
  ) -> DiffSignature {
    let lhs_tokens = tokenize_lines(lhs_lines);
    let rhs_tokens = tokenize_lines(rhs_lines);
    let token_diffs = diff(&lhs_tokens, &rhs_tokens, mode);

    let mut lhs_hasher = DefaultHasher::new();
    let mut rhs_hasher = DefaultHasher::new();

    for item in &token_diffs {
      if let DiffItem::Mutation { lhs: tl, rhs: tr } = item {
        for &tok in &lhs_tokens[tl.clone()] {
          NormalizedBytes::new(tok, mode).hash(&mut lhs_hasher);
        }
        for &tok in &rhs_tokens[tr.clone()] {
          NormalizedBytes::new(tok, mode).hash(&mut rhs_hasher);
        }
      }
    }

    DiffSignature {
      lhs: lhs_hasher.finish(),
      rhs: rhs_hasher.finish(),
    }
  }
}

/// A mutation that belongs to a cluster.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ClusterEntry {
  pub lhs_path: Option<PathBuf>,
  pub rhs_path: Option<PathBuf>,
}

/// A group of mutations that all share the same diff signature.
#[derive(Debug)]
pub struct DiffCluster {
  pub signature: DiffSignature,
  pub entries: BTreeMap<ClusterEntry, usize>,
  pub exemplar_lhs: Vec<u8>,
  pub exemplar_rhs: Vec<u8>,
}

impl DiffCluster {
  /// Group files into clusters of mutations, sorted by cluster size.
  pub fn cluster(
    events: impl Iterator<Item = FilePairEvent>,
    mode: IgnoreWhitespace,
  ) -> Vec<DiffCluster> {
    let mut map: HashMap<DiffSignature, DiffCluster> = HashMap::new();

    for event in events {
      if let FilePairEvent::TextDiff {
        lhs_path,
        rhs_path,
        lhs_content,
        rhs_content,
      } = event
      {
        let lhs_lines = split_lines(&lhs_content);
        let rhs_lines = split_lines(&rhs_content);
        let line_diffs = diff(&lhs_lines, &rhs_lines, mode);

        for item in &line_diffs {
          if let DiffItem::Mutation { lhs, rhs } = item {
            let lhs = &lhs_lines[lhs.clone()];
            let rhs = &rhs_lines[rhs.clone()];
            let sig = DiffSignature::new(lhs, rhs, mode);
            let cluster =
              map.entry(sig.clone()).or_insert_with(|| DiffCluster {
                signature: sig,
                entries: BTreeMap::new(),
                exemplar_lhs: lines_to_bytes(lhs),
                exemplar_rhs: lines_to_bytes(rhs),
              });
            *cluster
              .entries
              .entry(ClusterEntry {
                lhs_path: lhs_path.clone(),
                rhs_path: rhs_path.clone(),
              })
              .or_insert(0) += 1;
          }
        }
      }
    }

    let mut clusters: Vec<DiffCluster> = map.into_values().collect();
    clusters.sort_by(|a, b| {
      let a_total: usize = a.entries.values().sum();
      let b_total: usize = b.entries.values().sum();
      b_total.cmp(&a_total)
    });
    clusters
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sig(lhs: &[u8], rhs: &[u8], mode: IgnoreWhitespace) -> DiffSignature {
    let lhs_lines = split_lines(lhs);
    let rhs_lines = split_lines(rhs);
    DiffSignature::new(&lhs_lines, &rhs_lines, mode)
  }

  fn sizes(clusters: &[DiffCluster]) -> Vec<usize> {
    clusters.iter().map(|c| c.entries.values().sum()).collect()
  }

  #[test]
  fn signature_whitespace_normalization() {
    assert_eq!(
      sig(b"x = 1\n", b"x  =  1\n", IgnoreWhitespace::LengthChanges),
      sig(b"a = b\n", b"a  =  b\n", IgnoreWhitespace::LengthChanges)
    );
  }

  #[test]
  fn signature_different_changes() {
    assert_ne!(
      sig(b"foo\n", b"bar\n", IgnoreWhitespace::No),
      sig(b"foo\n", b"baz\n", IgnoreWhitespace::No)
    );
  }

  #[test]
  fn signature_identical_content() {
    assert_eq!(
      sig(b"hello\n", b"hello\n", IgnoreWhitespace::No),
      sig(b"world\n", b"world\n", IgnoreWhitespace::No)
    );
  }

  #[test]
  fn group_clusters_basic() {
    let events = vec![
      FilePairEvent::TextDiff {
        lhs_path: Some("a/old.txt".into()),
        rhs_path: Some("a/new.txt".into()),
        lhs_content: b"foo\n".to_vec(),
        rhs_content: b"bar\n".to_vec(),
      },
      FilePairEvent::TextDiff {
        lhs_path: Some("b/old.txt".into()),
        rhs_path: Some("b/new.txt".into()),
        lhs_content: b"foo\n".to_vec(),
        rhs_content: b"bar\n".to_vec(),
      },
      FilePairEvent::TextDiff {
        lhs_path: Some("c/old.txt".into()),
        rhs_path: Some("c/new.txt".into()),
        lhs_content: b"hello\n".to_vec(),
        rhs_content: b"world\n".to_vec(),
      },
    ];

    let clusters =
      DiffCluster::cluster(events.into_iter(), IgnoreWhitespace::No);
    assert_eq!(sizes(&clusters), [2, 1]);
  }

  #[test]
  fn group_clusters_per_mutation() {
    // A single file with two identical foo -> bar mutations should contribute
    // two entries to the same cluster, not one entry per file.
    let events = vec![FilePairEvent::TextDiff {
      lhs_path: Some("f.txt".into()),
      rhs_path: Some("f.txt".into()),
      lhs_content: b"foo\nkeep\nfoo\n".to_vec(),
      rhs_content: b"bar\nkeep\nbar\n".to_vec(),
    }];

    let clusters =
      DiffCluster::cluster(events.into_iter(), IgnoreWhitespace::No);
    assert_eq!(sizes(&clusters), [2]);
  }
}
