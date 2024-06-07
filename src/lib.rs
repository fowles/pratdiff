use std::collections::HashMap;
use std::iter::zip;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

use DiffItem::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hunk {
  diffs: Vec<DiffItem>,
}

impl Hunk {
  pub fn build(context: usize, diffs: &[DiffItem]) -> Vec<Hunk> {
    let mut res = vec![Hunk { diffs: Vec::new() }];

    for d in diffs {
      res.last_mut().unwrap().diffs.push(*d);

      if matches!(d, Match { len, .. } if *len > 2 * context) {
        res.push(Hunk { diffs: vec![*d] });
      }
    }

    res
      .into_iter()
      .filter_map(|mut hunk| {
        if hunk.diffs.len() <= 1 {
          return None;
        }

        if context == 0 {
          hunk.diffs.retain(|&d| matches!(d, Mutation { .. }));
          return Some(hunk);
        }

        if let Some(Match { lhs, rhs, len }) = hunk.diffs.first_mut() {
          if *len > context {
            *lhs += *len - context;
            *rhs += *len - context;
            *len = context;
          }
        }
        if let Some(Match { lhs, rhs, len }) = hunk.diffs.last_mut() {
          if *len > context {
            *lhs += *len - context;
            *rhs += *len - context;
            *len = context;
          }
        }
        Some(hunk)
      })
      .collect()
  }

  pub fn lhs_pos(&self) -> usize {
    match self.diffs.first() {
      Some(Match { lhs, .. }) => *lhs,
      Some(Mutation { lhs_pos, .. }) => *lhs_pos,
      _ => 0,
    }
  }

  pub fn lhs_len(&self) -> usize {
    self
      .diffs
      .iter()
      .map(|&d| match d {
        Match { len, .. } => len,
        Mutation { lhs_len, .. } => lhs_len,
      })
      .sum()
  }

  pub fn rhs_pos(&self) -> usize {
    match self.diffs.first() {
      Some(Match { rhs, .. }) => *rhs,
      Some(Mutation { rhs_pos, .. }) => *rhs_pos,
      _ => 0,
    }
  }

  pub fn rhs_len(&self) -> usize {
    self
      .diffs
      .iter()
      .map(|&d| match d {
        Match { len, .. } => len,
        Mutation { rhs_len, .. } => rhs_len,
      })
      .sum()
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
    if let Some(Match { len: match_len, .. }) = self.vec.last_mut() {
      *match_len += len;
    } else {
      self.vec.push(Match {
        lhs: self.lhs_pos(),
        rhs: self.rhs_pos(),
        len,
      });
    }
  }

  fn add_mutation(&mut self, lhs: usize, rhs: usize) {
    if lhs == 0 && rhs == 0 {
      return;
    }
    if let Some(Mutation { lhs_len, rhs_len, .. }) = self.vec.last_mut() {
      *lhs_len += lhs;
      *rhs_len += rhs;
    } else {
      self.vec.push(Mutation {
        lhs_pos: self.lhs_pos(),
        lhs_len: lhs,
        rhs_pos: self.rhs_pos(),
        rhs_len: rhs,
      });
    }
  }

  fn lhs_pos(&self) -> usize {
    match self.vec.last() {
      None => 0,
      Some(Match { lhs, len, .. }) => lhs + len,
      Some(Mutation { lhs_pos, lhs_len, .. }) => lhs_pos + lhs_len,
    }
  }

  fn rhs_pos(&self) -> usize {
    match self.vec.last() {
      None => 0,
      Some(Match { rhs, len, .. }) => rhs + len,
      Some(Mutation { rhs_pos, rhs_len, .. }) => rhs_pos + rhs_len,
    }
  }
}

pub fn diff_lines(lhs: &str, rhs: &str) -> Vec<DiffItem> {
  let lhs_lines: Vec<_> = lhs.lines().collect();
  let rhs_lines: Vec<_> = rhs.lines().collect();
  diff(&lhs_lines, &rhs_lines)
}

pub fn diff(lhs: &[&str], rhs: &[&str]) -> Vec<DiffItem> {
  let mut d = Diffs::default();
  accumulate_partitions(&mut d, &lhs, &rhs);
  d.vec
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
fn accumulate_diffs(diffs: &mut Diffs, lhs: &[&str], rhs: &[&str]) {
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

fn leading_match_len(lhs: &[&str], rhs: &[&str]) -> usize {
  zip(lhs, rhs).take_while(|(&l, &r)| l == r).count()
}

fn trailing_match_len(lhs: &[&str], rhs: &[&str]) -> usize {
  zip(lhs.iter().rev(), rhs.iter().rev())
    .take_while(|(&l, &r)| l == r)
    .count()
}

fn accumulate_partitions(diffs: &mut Diffs, lhs: &[&str], rhs: &[&str]) {
  let matched = (1..5) // 5 selected arbitrarily
    .filter_map(|i| match_lines(i, lhs, rhs))
    .next()
    .unwrap_or_default();
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

fn match_lines(
  arity: usize,
  lhs: &[&str],
  rhs: &[&str],
) -> Option<Vec<(usize, usize)>> {
  let mut m: HashMap<&str, (Vec<usize>, Vec<usize>)> = HashMap::new();
  for (i, l) in lhs.iter().enumerate() {
    m.entry(l).or_default().0.push(i);
  }
  for (i, r) in rhs.iter().enumerate() {
    m.entry(r).or_default().1.push(i);
  }

  let mut v: Vec<(usize, usize)> = m
    .into_values()
    .filter(|(l, r)| l.len() == arity && r.len() == arity)
    .map(|(l, r)| zip(l, r))
    .flatten()
    .collect();
  v.sort();
  if v.is_empty() {
    None
  } else {
    Some(v)
  }
}

fn longest_common_subseq(pairings: &[(usize, usize)]) -> Vec<(usize, usize)> {
  type PairingStack = Vec<Vec<((usize, usize), usize)>>;
  fn find_push_pos(stacks: &PairingStack, p: &(usize, usize)) -> usize {
    for (pos, stack) in stacks.iter().enumerate() {
      if p.1 < stack.last().unwrap().0 .1 {
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
  use super::*;

  #[test]
  fn diff_empty() {
    assert_eq!(diff(&[], &[]), vec![]);
  }

  #[test]
  fn diff_eq() {
    assert_eq!(
      diff(&["a", "b", "c"], &["a", "b", "c"]),
      vec![Match { lhs: 0, rhs: 0, len: 3 }]
    );
  }

  #[test]
  fn diff_ne() {
    assert_eq!(
      diff(&["a", "b", "c"], &["a", "c"]),
      vec![
        Match { lhs: 0, rhs: 0, len: 1 },
        Mutation {
          lhs_pos: 1,
          lhs_len: 1,
          rhs_pos: 1,
          rhs_len: 0,
        },
        Match { lhs: 2, rhs: 1, len: 1 },
      ]
    );
    assert_eq!(
      diff(&["z", "a", "b", "c"], &["a", "c"]),
      vec![
        Mutation {
          lhs_pos: 0,
          lhs_len: 1,
          rhs_pos: 0,
          rhs_len: 0,
        },
        Match { lhs: 1, rhs: 0, len: 1 },
        Mutation {
          lhs_pos: 2,
          lhs_len: 1,
          rhs_pos: 1,
          rhs_len: 0,
        },
        Match { lhs: 3, rhs: 1, len: 1 },
      ]
    );
    assert_eq!(
      diff(&["z", "a", "e", "b", "c"], &["a", "e", "c"]),
      vec![
        Mutation {
          lhs_pos: 0,
          lhs_len: 1,
          rhs_pos: 0,
          rhs_len: 0,
        },
        Match { lhs: 1, rhs: 0, len: 2 },
        Mutation {
          lhs_pos: 3,
          lhs_len: 1,
          rhs_pos: 2,
          rhs_len: 0,
        },
        Match { lhs: 4, rhs: 2, len: 1 },
      ]
    );
  }

  #[test]
  fn diff_only_non_unique() {
    assert_eq!(
      diff(&["a", "b", "b", "c"], &["b", "b"]),
      vec![
        Mutation {
          lhs_pos: 0,
          lhs_len: 1,
          rhs_pos: 0,
          rhs_len: 0,
        },
        Match { lhs: 1, rhs: 0, len: 2 },
        Mutation {
          lhs_pos: 3,
          lhs_len: 1,
          rhs_pos: 2,
          rhs_len: 0,
        },
      ]
    );
  }

  #[test]
  fn match_lines_arity1() {
    assert_eq!(
      match_lines(1, &["a", "b", "c", "d", "e", "d"], &["a", "c", "d", "e"]),
      Some(vec![(0, 0), (2, 1), (4, 3)]),
    );
  }

  #[test]
  fn match_lines_arity2() {
    assert_eq!(
      match_lines(2, &["a", "b", "b", "c"], &["b", "b"]),
      Some(vec![(1, 0), (2, 1)]),
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
      vec![(1, 4), (2, 6), (5, 7), (8, 10), (9, 11), (12, 13),]
    );
  }

  #[test]
  fn lead_trail_overlap() {
    assert_eq!(
      diff(&["a", "b", "d", "b", "c"], &["a", "b", "c"]),
      vec![
        Match { lhs: 0, rhs: 0, len: 2 },
        Mutation {
          lhs_pos: 2,
          lhs_len: 2,
          rhs_pos: 2,
          rhs_len: 0,
        },
        Match { lhs: 4, rhs: 2, len: 1 },
      ]
    );
  }

  #[test]
  fn lead_move_txt() {
    assert_eq!(
      diff_lines(
        include_str!("testdata/move.v0.txt"),
        include_str!("testdata/move.v1.txt"),
      ),
      vec![
        Mutation {
          lhs_pos: 0,
          lhs_len: 8,
          rhs_pos: 0,
          rhs_len: 0,
        },
        Match { lhs: 8, rhs: 0, len: 8 },
        Mutation {
          lhs_pos: 16,
          lhs_len: 0,
          rhs_pos: 8,
          rhs_len: 8,
        },
      ]
    );
  }

  fn hunk_positions(hunks: &[Hunk]) -> Vec<((usize, usize), (usize, usize))> {
    hunks
      .iter()
      .map(|h| ((h.lhs_pos() + 1, h.lhs_len()), (h.rhs_pos() + 1, h.rhs_len())))
      .collect::<Vec<_>>()
  }

  #[test]
  fn build_hunks() {
    let diff = diff_lines(
      include_str!("testdata/move.v0.txt"),
      include_str!("testdata/move.v1.txt"),
    );
    assert_eq!(
      hunk_positions(&Hunk::build(3, &diff)),
      vec![((1, 11), (1, 3)), ((14, 3), (6, 11))]
    );
    assert_eq!(
      hunk_positions(&Hunk::build(0, &diff)),
      vec![((1, 8), (1, 0)), ((17, 0), (9, 8))]
    );
  }
}
