use std::collections::HashMap;
use std::iter::zip;
use std::iter::Iterator;
use unicode_segmentation::UnicodeSegmentation;

mod diff;
mod files;
mod printer;
mod style;

pub use diff::DiffItem;
use diff::Diffs;

pub use files::diff_files;
pub use printer::Printer;
pub use style::Styles;

struct ByteTokenIter<'a> {
  content: &'a [u8],
  valid: &'a str,
}

impl<'a> ByteTokenIter<'a> {
  fn new(content: &'a [u8]) -> Self {
    ByteTokenIter { content, valid: valid_prefix(content) }
  }
}

fn valid_prefix(content: &[u8]) -> &str {
  match std::str::from_utf8(content) {
    Ok(s) => s,
    Err(e) => std::str::from_utf8(&content[..e.valid_up_to()]).unwrap(),
  }
}

impl<'a> Iterator for ByteTokenIter<'a> {
  type Item = &'a [u8];

  fn next(&mut self) -> Option<&'a [u8]> {
    if self.content.is_empty() {
      return None;
    }

    // If the valid prefix is exhausted we're sitting on an invalid sequence.
    // Emit it as a single opaque token, then recompute the next valid prefix.
    if self.valid.is_empty() {
      let n = std::str::from_utf8(self.content)
        .err()
        .and_then(|e| e.error_len())
        .unwrap_or(1);
      let (token, rest) = self.content.split_at(n);
      self.content = rest;
      self.valid = valid_prefix(self.content);
      return Some(token);
    }

    let mut graphemes = self.valid.grapheme_indices(true);
    let (_, first) = graphemes.next().unwrap();
    let first_char = first.chars().next().unwrap();

    let scan = |pred: &dyn Fn(char) -> bool| {
      let mut end = first.len();
      for (off, g) in graphemes {
        if g.chars().next().map_or(false, pred) {
          end = off + g.len();
        } else {
          break;
        }
      }
      end
    };

    let end = if first_char.is_ascii_whitespace() {
      scan(&|c| c.is_ascii_whitespace())
    } else if first_char.is_ascii_digit() {
      scan(&|c| c.is_ascii_digit())
    } else if first_char.is_ascii_alphabetic() || first_char == '_' {
      scan(&|c| c.is_ascii_alphanumeric() || c == '_')
    } else {
      first.len() // single ASCII symbol or non-ASCII grapheme cluster
    };

    let (token, rest) = self.content.split_at(end);
    self.content = rest;
    self.valid = &self.valid[end..];
    Some(token)
  }
}

pub fn diff(lhs: &[&[u8]], rhs: &[&[u8]]) -> Vec<DiffItem> {
  let mut d = Diffs::default();
  accumulate_partitions(&mut d, lhs, rhs);
  d.vec
}

pub fn tokenize_lines<'a>(lines: &[&'a [u8]]) -> Vec<&'a [u8]> {
  let mut v: Vec<_> = lines
    .iter()
    .flat_map(|l| ByteTokenIter::new(l).chain([b"\n" as &[u8]]))
    .collect();
  v.pop();
  v
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
  use diff::DiffItem;
  use diff::DiffItem::*;
  use diff::Hunk;
  use std::ops::Range;

  fn diff_lines(lhs: &[u8], rhs: &[u8]) -> Vec<DiffItem> {
    let lhs_lines: Vec<_> = split_lines(lhs);
    let rhs_lines: Vec<_> = split_lines(rhs);
    diff(&lhs_lines, &rhs_lines)
  }

  fn split_lines(content: &[u8]) -> Vec<&[u8]> {
    if content.is_empty() {
      return vec![];
    }
    let content = content.strip_suffix(b"\n").unwrap_or(content);
    content.split(|b| *b == b'\n').collect()
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

  fn hunk_positions(hunks: &[Hunk]) -> Vec<((usize, usize), (usize, usize))> {
    hunks
      .iter()
      .map(|h| {
        let (l, r) = (h.lhs(), h.rhs());
        ((l.start + 1, l.len()), (r.start + 1, r.len()))
      })
      .collect::<Vec<_>>()
  }

  #[test]
  fn build_hunks() {
    let diff = diff_lines(
      include_bytes!("testdata/old/move.txt"),
      include_bytes!("testdata/new/move.txt"),
    );
    assert_eq!(
      hunk_positions(&Hunk::build(3, &diff)),
      &[((1, 11), (1, 3)), ((14, 3), (6, 11))]
    );
    assert_eq!(
      hunk_positions(&Hunk::build(0, &diff)),
      &[((1, 8), (1, 0)), ((17, 0), (9, 8))]
    );
  }

  #[test]
  fn tokenize_invalid_utf8() {
    // 0xFF is not valid UTF-8; should be its own token between "foo" and "bar"
    assert_eq!(
      tokenize_lines(&[b"foo\xffbar"]),
      &[b"foo" as &[u8], b"\xff", b"bar"],
    );
  }

  #[test]
  fn tokenize_combining_diacritics() {
    // "café" with decomposed é: e (U+0065) + combining acute accent (U+0301 = 0xCC 0x81)
    // Should be one identifier token, not split at the combining mark.
    assert_eq!(
      tokenize_lines(&[b"cafe\xcc\x81"]),
      &[b"cafe\xcc\x81" as &[u8]],
    );
  }

  #[test]
  fn tokenize() {
    assert_eq!(
      tokenize_lines(&[b"void func1() {", b"  x += 1"]),
      &[
        b"void" as &[u8],
        b" ",
        b"func1",
        b"(",
        b")",
        b" ",
        b"{",
        b"\n",
        b"  ",
        b"x",
        b" ",
        b"+",
        b"=",
        b" ",
        b"1"
      ],
    );
  }
}
