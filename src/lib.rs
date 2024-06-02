#![allow(unused)] // TODO(kfm): remove this

use std::cmp;

#[derive(PartialEq, Eq, Clone, Debug)]
enum DiffItem {
  Match {
    lhs: usize,
    rhs: usize,
    len: usize,
  },
  Mutation {
    lhs_pos: usize,
    lhs_len: usize,
    rhs_pos: usize,
    rhs_len: usize,
  },
}

fn leading_match_len(lhs: &[&str], rhs: &[&str]) -> usize {
  for n in 0..cmp::min(lhs.len(), rhs.len()) {
    if lhs[n] != rhs[n] {
      return n;
    }
  }
  cmp::min(lhs.len(), rhs.len())
}

fn trailing_match_len(lhs: &[&str], rhs: &[&str]) -> usize {
  for n in 0..cmp::min(lhs.len(), rhs.len()) {
    if lhs[lhs.len() - 1 - n] != rhs[rhs.len() - 1 - n] {
      return n;
    }
  }
  cmp::min(lhs.len(), rhs.len())
}

// Patience diff algorithm
//
// 1. Match the first lines of both if they're identical, then match the second,
//    third, etc. until a pair doesn't match.
// 2. Match the last lines of both if they're identical, then match the next to
//    last, second to last, etc. until a pair doesn't match.
// 3. Find all lines which occur exactly once on both sides, then do longest
//    common subsequence on those lines, matching them up.
// 4. Do steps 1-2 on each section between matched lines.
fn diff(lhs: &[&str], rhs: &[&str]) -> Vec<DiffItem> {
  let mut r = Vec::new();
  let leading = leading_match_len(lhs, rhs);
  if leading != 0 {
    r.push(DiffItem::Match { lhs: 0, rhs: 0, len: leading });
  }

  if leading == lhs.len() && leading == rhs.len() {
    return r;
  }

  let trailing = trailing_match_len(lhs, rhs);

  // TODO(kfm): step 3
  r.push(DiffItem::Mutation {
    lhs_pos: leading,
    lhs_len: lhs.len() - leading - trailing,
    rhs_pos: leading,
    rhs_len: rhs.len() - leading - trailing,
  });

  if trailing != 0 {
    r.push(DiffItem::Match {
      lhs: lhs.len() - trailing,
      rhs: rhs.len() - trailing,
      len: trailing,
    });
  }

  r
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty() {
    assert_eq!(diff(&vec![], &vec![]), vec![]);
  }

  #[test]
  fn identical() {
    assert_eq!(
      diff(&vec!["a", "b", "c"], &vec!["a", "b", "c"]),
      vec![DiffItem::Match { lhs: 0, rhs: 0, len: 3 }]
    );
  }

  #[test]
  fn different() {
    assert_eq!(
      diff(&vec!["a", "b", "c"], &vec!["a", "c"]),
      vec![
        DiffItem::Match { lhs: 0, rhs: 0, len: 1 },
        DiffItem::Mutation {
          lhs_pos: 1,
          lhs_len: 1,
          rhs_pos: 1,
          rhs_len: 0,
        },
        DiffItem::Match { lhs: 2, rhs: 1, len: 1 },
      ]
    );
  }
}
