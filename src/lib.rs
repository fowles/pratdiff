#![allow(unused)] // TODO(kfm): remove this

use std::collections::HashMap;
use std::iter::zip;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DiffItem {
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

// Patience diff algorithm
//
// 1. Match the first lines of both if they're identical, then match the second,
//    third, etc. until a pair doesn't match.
// 2. Match the last lines of both if they're identical, then match the next to
//    last, second to last, etc. until a pair doesn't match.
// 3. Find all lines which occur exactly once on both sides, then do longest
//    common subsequence on those lines, matching them up.
// 4. Do steps 1-2 on each section between matched lines.
pub fn diff(lhs: &[&str], rhs: &[&str]) -> Vec<DiffItem> {
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

fn leading_match_len(lhs: &[&str], rhs: &[&str]) -> usize {
  zip(lhs, rhs).take_while(|(&l, &r)| l == r).count()
}

fn trailing_match_len(lhs: &[&str], rhs: &[&str]) -> usize {
  zip(lhs.iter().rev(), rhs.iter().rev())
    .take_while(|(&l, &r)| l == r)
    .count()
}

fn partition(lhs: &[&str], rhs: &[&str]) -> Vec<DiffItem> {
  let mut r = vec![];
  if lhs.is_empty() && rhs.is_empty() {
    // nothing
  } else if (lhs.is_empty()) {
    r.push(DiffItem::Mutation {
      lhs_pos: 0,
      lhs_len: 0,
      rhs_pos: 0,
      rhs_len: rhs.len(),
    });
  } else if (rhs.is_empty()) {
    r.push(DiffItem::Mutation {
      lhs_pos: 0,
      lhs_len: lhs.len(),
      rhs_pos: 0,
      rhs_len: 0,
    });
  } else {
    let matched = match_unique_lines(lhs, rhs);

  }
  r
}

fn match_unique_lines(lhs: &[&str], rhs: &[&str]) -> Vec<(usize, usize)> {
  let mut m = HashMap::<&str, (Vec<usize>, Vec<usize>)>::new();
  for (i, l) in lhs.iter().enumerate() {
    m.entry(l).or_default().0.push(i);
  }
  for (i, r) in rhs.iter().enumerate() {
    m.entry(r).or_default().1.push(i);
  }

  let mut v: Vec<(usize, usize)> = m
    .iter()
    .map(|(_, v)| v)
    .filter(|(l, r)| l.len() == 1 && r.len() == 1)
    .map(|(l, r)| (l[0], r[0]))
    .collect();
  v.sort();
  v
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn diff_empty() {
    assert_eq!(diff(&vec![], &vec![]), vec![]);
  }

  #[test]
  fn diff_eq() {
    assert_eq!(
      diff(&vec!["a", "b", "c"], &vec!["a", "b", "c"]),
      vec![DiffItem::Match { lhs: 0, rhs: 0, len: 3 }]
    );
  }

  #[test]
  fn diff_ne() {
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

  #[test]
  fn match_unique_lines_basic() {
    assert_eq!(
      match_unique_lines(
        &vec!["a", "b", "c", "d", "e", "d",],
        &vec!["a", "c", "d", "e"]
      ),
      vec![(0, 0), (2, 1), (4, 3)]
    );
  }
}
