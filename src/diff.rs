use std::collections::HashMap;
use std::iter::zip;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
  Lhs,
  Rhs,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiffItem {
  Match { lhs: Range<usize>, rhs: Range<usize> },
  Mutation { lhs: Range<usize>, rhs: Range<usize> },
}

use DiffItem::*;

impl DiffItem {
  pub fn lhs(&self) -> Range<usize> {
    match self {
      Match { lhs, .. } => lhs.clone(),
      Mutation { lhs, .. } => lhs.clone(),
    }
  }

  pub fn rhs(&self) -> Range<usize> {
    match self {
      Match { rhs, .. } => rhs.clone(),
      Mutation { rhs, .. } => rhs.clone(),
    }
  }

  pub fn side(&self, side: Side) -> Range<usize> {
    match side {
      Side::Lhs => self.lhs(),
      Side::Rhs => self.rhs(),
    }
  }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Diffs {
  vec: Vec<DiffItem>,
}

impl Diffs {
  fn add_match(&mut self, len: usize) {
    if len == 0 {
      return;
    }
    if let Some(Match { lhs, rhs }) = self.vec.last_mut() {
      lhs.end += len;
      rhs.end += len;
    } else {
      self.vec.push(Match {
        lhs: Range {
          start: self.lhs_pos(),
          end: self.lhs_pos() + len,
        },
        rhs: Range {
          start: self.rhs_pos(),
          end: self.rhs_pos() + len,
        },
      });
    }
  }

  fn add_mutation(&mut self, lhs: usize, rhs: usize) {
    if lhs == 0 && rhs == 0 {
      return;
    }
    if let Some(Mutation { lhs: l, rhs: r }) = self.vec.last_mut() {
      l.end += lhs;
      r.end += rhs;
    } else {
      self.vec.push(Mutation {
        lhs: Range {
          start: self.lhs_pos(),
          end: self.lhs_pos() + lhs,
        },
        rhs: Range {
          start: self.rhs_pos(),
          end: self.rhs_pos() + rhs,
        },
      });
    }
  }

  fn lhs_pos(&self) -> usize {
    self.vec.last().map_or(0, |d| d.lhs().end)
  }

  fn rhs_pos(&self) -> usize {
    self.vec.last().map_or(0, |d| d.rhs().end)
  }
}

// --- Patience Diff algorithm ---
//
// 1. Match the first lines of both if they're identical, then match the second,
//    third, etc. until a pair doesn't match.
// 2. Match the last lines of both if they're identical, then match the next to
//    last, second to last, etc. until a pair doesn't match.
// 3. Find all lines which occur exactly once on both sides, then do longest
//    common subsequence on those lines, matching them up.
// 4. Do steps 1-2 on each section between matched lines.

pub fn diff(lhs: &[&[u8]], rhs: &[&[u8]]) -> Vec<DiffItem> {
  let mut d = Diffs::default();
  accumulate_partitions(&mut d, lhs, rhs);
  d.vec
}

fn accumulate_diffs(diffs: &mut Diffs, lhs: &[&[u8]], rhs: &[&[u8]]) {
  let leading = leading_match_len(lhs, rhs);
  diffs.add_match(leading);
  if leading == lhs.len() && leading == rhs.len() {
    return;
  }

  let trailing =
    trailing_match_len(&lhs[leading..lhs.len()], &rhs[leading..rhs.len()]);
  accumulate_partitions(
    diffs,
    &lhs[leading..lhs.len() - trailing],
    &rhs[leading..rhs.len() - trailing],
  );
  diffs.add_match(trailing);
}

fn leading_match_len(lhs: &[&[u8]], rhs: &[&[u8]]) -> usize {
  zip(lhs, rhs).take_while(|(l, r)| l == r).count()
}

fn trailing_match_len(lhs: &[&[u8]], rhs: &[&[u8]]) -> usize {
  zip(lhs.iter().rev(), rhs.iter().rev())
    .take_while(|(l, r)| l == r)
    .count()
}

fn accumulate_partitions(diffs: &mut Diffs, lhs: &[&[u8]], rhs: &[&[u8]]) {
  let matched = match_lines(lhs, rhs);
  if matched.is_empty() {
    diffs.add_mutation(lhs.len(), rhs.len());
    return;
  }
  let matched = longest_common_subseq(&matched);

  let mut lhs_pos: usize = 0;
  let mut rhs_pos: usize = 0;
  for (lhs_next, rhs_next) in matched {
    accumulate_diffs(diffs, &lhs[lhs_pos..lhs_next], &rhs[rhs_pos..rhs_next]);
    diffs.add_match(1);
    lhs_pos = lhs_next + 1;
    rhs_pos = rhs_next + 1;
  }
  accumulate_diffs(diffs, &lhs[lhs_pos..lhs.len()], &rhs[rhs_pos..rhs.len()]);
}

fn match_lines(lhs: &[&[u8]], rhs: &[&[u8]]) -> Vec<(usize, usize)> {
  let mut m: HashMap<&[u8], (Vec<usize>, Vec<usize>)> = HashMap::new();
  for (i, l) in lhs.iter().enumerate() {
    m.entry(l).or_default().0.push(i);
  }
  for (i, r) in rhs.iter().enumerate() {
    m.entry(r).or_default().1.push(i);
  }

  let mut min = usize::MAX;
  m.retain(|_, (l, r)| {
    if l.len() == r.len() {
      min = min.min(l.len());
      true
    } else {
      false
    }
  });

  let mut v: Vec<(usize, usize)> = m
    .into_values()
    .filter(|(l, _)| l.len() == min)
    .flat_map(|(l, r)| zip(l, r))
    .collect();
  v.sort();
  v
}

fn longest_common_subseq(pairings: &[(usize, usize)]) -> Vec<(usize, usize)> {
  type PairingStack = Vec<Vec<((usize, usize), usize)>>;
  fn find_push_pos(stacks: &PairingStack, p: &(usize, usize)) -> usize {
    for (pos, stack) in stacks.iter().enumerate() {
      if p.1 < stack.last().unwrap().0.1 {
        return pos;
      }
    }
    stacks.len()
  }

  let mut stacks = PairingStack::new();
  for p in pairings {
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
  use std::ops::Range;

  use super::*;
  use crate::tokens::split_lines;

  fn diff_lines(lhs: &[u8], rhs: &[u8]) -> Vec<DiffItem> {
    let lhs_lines: Vec<_> = split_lines(lhs);
    let rhs_lines: Vec<_> = split_lines(rhs);
    diff(&lhs_lines, &rhs_lines)
  }

  #[test]
  fn diff_empty() {
    assert_eq!(diff(&[] as &[&[u8]], &[]), &[]);
  }

  #[test]
  fn diff_eq() {
    assert_eq!(
      diff(&[b"a", b"b", b"c"], &[b"a", b"b", b"c"]),
      &[Match {
        lhs: Range { start: 0, end: 3 },
        rhs: Range { start: 0, end: 3 },
      }]
    );
  }

  #[test]
  fn diff_ne() {
    assert_eq!(
      diff(&[b"a", b"b", b"c"], &[b"a", b"c"]),
      &[
        Match {
          lhs: Range { start: 0, end: 1 },
          rhs: Range { start: 0, end: 1 },
        },
        Mutation {
          lhs: Range { start: 1, end: 2 },
          rhs: Range { start: 1, end: 1 },
        },
        Match {
          lhs: Range { start: 2, end: 3 },
          rhs: Range { start: 1, end: 2 },
        },
      ]
    );
    assert_eq!(
      diff(&[b"z", b"a", b"b", b"c"], &[b"a", b"c"]),
      &[
        Mutation {
          lhs: Range { start: 0, end: 1 },
          rhs: Range { start: 0, end: 0 },
        },
        Match {
          lhs: Range { start: 1, end: 2 },
          rhs: Range { start: 0, end: 1 },
        },
        Mutation {
          lhs: Range { start: 2, end: 3 },
          rhs: Range { start: 1, end: 1 },
        },
        Match {
          lhs: Range { start: 3, end: 4 },
          rhs: Range { start: 1, end: 2 },
        },
      ]
    );
    assert_eq!(
      diff(&[b"z", b"a", b"e", b"b", b"c"], &[b"a", b"e", b"c"]),
      &[
        Mutation {
          lhs: Range { start: 0, end: 1 },
          rhs: Range { start: 0, end: 0 },
        },
        Match {
          lhs: Range { start: 1, end: 3 },
          rhs: Range { start: 0, end: 2 },
        },
        Mutation {
          lhs: Range { start: 3, end: 4 },
          rhs: Range { start: 2, end: 2 },
        },
        Match {
          lhs: Range { start: 4, end: 5 },
          rhs: Range { start: 2, end: 3 },
        },
      ]
    );
  }

  #[test]
  fn diff_only_non_unique() {
    assert_eq!(
      diff(&[b"a", b"b", b"b", b"c"], &[b"b", b"b"]),
      &[
        Mutation {
          lhs: Range { start: 0, end: 1 },
          rhs: Range { start: 0, end: 0 },
        },
        Match {
          lhs: Range { start: 1, end: 3 },
          rhs: Range { start: 0, end: 2 },
        },
        Mutation {
          lhs: Range { start: 3, end: 4 },
          rhs: Range { start: 2, end: 2 },
        },
      ]
    );
  }

  #[test]
  fn match_lines_arity1() {
    assert_eq!(
      match_lines(
        &[b"a", b"b", b"c", b"d", b"e", b"d"],
        &[b"a", b"c", b"d", b"e"]
      ),
      vec![(0, 0), (2, 1), (4, 3)],
    );
  }

  #[test]
  fn match_lines_arity2() {
    assert_eq!(
      match_lines(&[b"a", b"b", b"b", b"c"], &[b"b", b"b"]),
      vec![(1, 0), (2, 1)],
    );
  }

  #[test]
  fn longest_common_subseq_basic() {
    // From https://blog.jcoglan.com/2017/09/19/the-patience-diff-algorithm/
    assert_eq!(
      longest_common_subseq(&[
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
      &[(1, 4), (2, 6), (5, 7), (8, 10), (9, 11), (12, 13),]
    );
  }

  #[test]
  fn lead_trail_overlap() {
    assert_eq!(
      diff(&[b"a", b"b", b"d", b"b", b"c"], &[b"a", b"b", b"c"]),
      &[
        Match {
          lhs: Range { start: 0, end: 2 },
          rhs: Range { start: 0, end: 2 },
        },
        Mutation {
          lhs: Range { start: 2, end: 4 },
          rhs: Range { start: 2, end: 2 },
        },
        Match {
          lhs: Range { start: 4, end: 5 },
          rhs: Range { start: 2, end: 3 },
        },
      ]
    );
  }

  #[test]
  fn lead_move_txt() {
    assert_eq!(
      diff_lines(
        include_bytes!("testdata/old/move.txt"),
        include_bytes!("testdata/new/move.txt"),
      ),
      &[
        Mutation {
          lhs: Range { start: 0, end: 8 },
          rhs: Range { start: 0, end: 0 },
        },
        Match {
          lhs: Range { start: 8, end: 16 },
          rhs: Range { start: 0, end: 8 },
        },
        Mutation {
          lhs: Range { start: 16, end: 16 },
          rhs: Range { start: 8, end: 16 },
        },
      ]
    );
  }
}
