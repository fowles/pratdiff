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

impl DiffItem {
  fn offset(&self, l: usize, r: usize) -> DiffItem {
    match self {
      DiffItem::Match { lhs, rhs, len } => {
        DiffItem::Match { lhs: *lhs + l, rhs: *rhs + r, len: *len }
      }
      DiffItem::Mutation { lhs_pos, lhs_len, rhs_pos, rhs_len } => {
        DiffItem::Mutation {
          lhs_pos: *lhs_pos + l,
          lhs_len: *lhs_len,
          rhs_pos: *rhs_pos + r,
          rhs_len: *rhs_len,
        }
      }
    }
  }

  fn grow(&mut self, size: usize) {
    match self {
      DiffItem::Match { lhs, rhs, len } => {
        *len += size;
      }
      DiffItem::Mutation { lhs_pos, lhs_len, rhs_pos, rhs_len } => {
        *lhs_len += size;
        *rhs_len += size;
      }
    }
  }
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

  r.extend(
    partition(
      &lhs[leading..lhs.len() - trailing],
      &rhs[leading..rhs.len() - trailing],
    )
    .iter()
    .map(|d| d.offset(leading, leading)),
  );

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
  let matched = match_unique_lines(lhs, rhs);
  if matched.is_empty() {
    return vec![DiffItem::Mutation {
      lhs_pos: 0,
      lhs_len: lhs.len(),
      rhs_pos: 0,
      rhs_len: rhs.len(),
    }];
  }
  let matched = longest_common_subseq(&matched);

  let mut r = Vec::<DiffItem>::new();
  let mut lhs_pos: usize = 0;
  let mut rhs_pos: usize = 0;
  for (lhs_next, rhs_next) in matched.iter() {
    if lhs_pos == *lhs_next && rhs_pos == *rhs_next {
      r.last_mut().unwrap().grow(1);
    } else {
      r.extend(
        diff(&lhs[lhs_pos..*lhs_next], &rhs[rhs_pos..*rhs_next])
          .into_iter()
          .map(|d| d.offset(lhs_pos, rhs_pos)),
      );
      r.push(DiffItem::Match { lhs: *lhs_next, rhs: *rhs_next, len: 1 });
    }
    lhs_pos = lhs_next + 1;
    rhs_pos = rhs_next + 1;
  }
  r.extend(
    diff(&lhs[lhs_pos..lhs.len()], &rhs[rhs_pos..rhs.len()])
      .iter()
      .map(|d| d.offset(lhs_pos, rhs_pos)),
  );
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
    .values()
    .filter(|(l, r)| l.len() == 1 && r.len() == 1)
    .map(|(l, r)| (l[0], r[0]))
    .collect();
  v.sort();
  v
}

fn longest_common_subseq(pairings: &[(usize, usize)]) -> Vec<(usize, usize)> {
  type PairingStack = Vec<Vec<((usize, usize), usize)>>;
  let find_push_pos = |stacks: &PairingStack, p: &(usize, usize)| -> usize {
    for (pos, stack) in stacks.iter().enumerate() {
      if p.1 < stack.last().unwrap().0 .1 {
        return pos;
      }
    }
    stacks.len()
  };

  let mut stacks = PairingStack::new();
  for p in pairings.iter() {
    let push_pos = find_push_pos(&stacks, p);
    if push_pos == stacks.len() {
      stacks.push(vec![]);
    }
    let prev = if push_pos == 0 { 0 } else { stacks[push_pos - 1].len() - 1 };
    stacks[push_pos].push((*p, prev));
  }

  let mut r = vec![];
  let mut prev = stacks.last().unwrap().len() - 1;
  for stack in stacks.iter().rev() {
    r.push(stack[prev].0);
    prev = stack[prev].1;
  }
  r.reverse();
  r
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
    assert_eq!(
      diff(&vec!["z", "a", "b", "c"], &vec!["a", "c"]),
      vec![
        DiffItem::Mutation {
          lhs_pos: 0,
          lhs_len: 1,
          rhs_pos: 0,
          rhs_len: 0,
        },
        DiffItem::Match { lhs: 1, rhs: 0, len: 1 },
        DiffItem::Mutation {
          lhs_pos: 2,
          lhs_len: 1,
          rhs_pos: 1,
          rhs_len: 0,
        },
        DiffItem::Match { lhs: 3, rhs: 1, len: 1 },
      ]
    );
    assert_eq!(
      diff(&vec!["z", "a", "e", "b", "c"], &vec!["a", "e", "c"]),
      vec![
        DiffItem::Mutation {
          lhs_pos: 0,
          lhs_len: 1,
          rhs_pos: 0,
          rhs_len: 0,
        },
        DiffItem::Match { lhs: 1, rhs: 0, len: 2 },
        DiffItem::Mutation {
          lhs_pos: 3,
          lhs_len: 1,
          rhs_pos: 2,
          rhs_len: 0,
        },
        DiffItem::Match { lhs: 4, rhs: 2, len: 1 },
      ]
    );
  }

  #[test]
  fn match_unique_lines_basic() {
    assert_eq!(
      match_unique_lines(
        &vec!["a", "b", "c", "d", "e", "d"],
        &vec!["a", "c", "d", "e"]
      ),
      vec![(0, 0), (2, 1), (4, 3)]
    );
  }

  #[test]
  fn longest_common_subseq_basic() {
    // From https://blog.jcoglan.com/2017/09/19/the-patience-diff-algorithm/
    assert_eq!(
      longest_common_subseq(&vec![
        (0, 9),
        (1, 4),
        (2, 6),
        (3, 12),
        (4, 8),
        (5, 7),
        (6, 1),
        (7, 5),
        (8, 10),
        (9, 11),
        (10, 3),
        (11, 2),
        (12, 13),
      ]),
      vec![(1, 4), (2, 6), (5, 7), (8, 10), (9, 11), (12, 13),]
    );
  }
}
