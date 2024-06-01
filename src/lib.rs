// 1. Match the first lines of both if they're identical, then match the second, third, etc. until
//    a pair doesn't match.
// 2. Match the last lines of both if they're identical, then match the next to last, second to
//    last, etc. until a pair doesn't match.
// 3. Find all lines which occur exactly once on both sides, then do longest common subsequence on
//    those lines, matching them up.
// 4. Do steps 1-2 on each section between matched lines.

#![allow(unused)] // TODO(kfm): remove this

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

fn diff(lhs: &Vec<&str>, rhs: &Vec<&str>) -> Vec<DiffItem> {
  return vec![];
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn basic() {
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
