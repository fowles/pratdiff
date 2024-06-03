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

impl DiffItem {
  fn offset(&self, l: usize, r: usize) -> DiffItem {
    let mut copy = *self;
    match &mut copy {
      DiffItem::Mutation { lhs_pos: lhs, rhs_pos: rhs, .. }
      | DiffItem::Match { lhs, rhs, .. } => {
        *lhs += l;
        *rhs += r;
      }
    };
    copy
  }

  fn grow(&mut self, size: usize) {
    match self {
      DiffItem::Match { len, .. } => {
        *len += size;
      }
      DiffItem::Mutation { lhs_len, rhs_len, .. } => {
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
  let matched = (1..5) // 5 selected arbitrarily
    .filter_map(|i| match_lines(i, lhs, rhs))
    .next()
    .unwrap_or_default();
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
  for &(lhs_next, rhs_next) in matched.iter() {
    if lhs_pos == lhs_next && rhs_pos == rhs_next {
      r.last_mut().unwrap().grow(1);
    } else {
      r.extend(
        diff(&lhs[lhs_pos..lhs_next], &rhs[rhs_pos..rhs_next])
          .into_iter()
          .map(|d| d.offset(lhs_pos, rhs_pos)),
      );
      r.push(DiffItem::Match { lhs: lhs_next, rhs: rhs_next, len: 1 });
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

fn match_lines(
  match_arity: usize,
  lhs: &[&str],
  rhs: &[&str],
) -> Option<Vec<(usize, usize)>> {
  let mut m = HashMap::<&str, (Vec<usize>, Vec<usize>)>::new();
  for (i, l) in lhs.iter().enumerate() {
    m.entry(l).or_default().0.push(i);
  }
  for (i, r) in rhs.iter().enumerate() {
    m.entry(r).or_default().1.push(i);
  }

  let mut v: Vec<(usize, usize)> = m
    .into_values()
    .filter(|(l, r)| l.len() == match_arity && r.len() == match_arity)
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
      vec![DiffItem::Match { lhs: 0, rhs: 0, len: 3 }]
    );
  }

  #[test]
  fn diff_ne() {
    assert_eq!(
      diff(&["a", "b", "c"], &["a", "c"]),
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
      diff(&["z", "a", "b", "c"], &["a", "c"]),
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
      diff(&["z", "a", "e", "b", "c"], &["a", "e", "c"]),
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
  fn diff_only_non_unique() {
    assert_eq!(
      diff(&["a", "b", "b", "c"], &["b", "b"]),
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
}
