use std::ops::Range;

use DiffItem::*;

use crate::diff::DiffItem;
use crate::diff::Side;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hunk {
  pub diffs: Vec<DiffItem>,
}

impl Hunk {
  pub fn build(context: usize, diffs: &[DiffItem]) -> Vec<Hunk> {
    let mut res = vec![Hunk { diffs: Vec::new() }];

    for d in diffs {
      res.last_mut().unwrap().diffs.push(d.clone());

      if matches!(d, Match { lhs, .. } if lhs.len() > 2 * context) {
        res.push(Hunk { diffs: vec![d.clone()] });
      }
    }

    res
      .into_iter()
      .filter_map(|mut hunk| {
        if hunk.diffs.is_empty() {
          return None;
        }
        if hunk.diffs.len() == 1 && matches!(hunk.diffs[0], Match { .. }) {
          return None;
        }

        if context == 0 {
          hunk.diffs.retain(|d| matches!(d, Mutation { .. }));
          return Some(hunk);
        }

        if let Some(Match { lhs, rhs }) = hunk.diffs.first_mut()
          && lhs.len() > context
        {
          lhs.start = lhs.end - context;
          rhs.start = rhs.end - context;
        }
        if let Some(Match { lhs, rhs }) = hunk.diffs.last_mut()
          && lhs.len() > context
        {
          lhs.end = lhs.start + context;
          rhs.end = rhs.start + context;
        }
        Some(hunk)
      })
      .collect()
  }

  pub fn side(&self, side: Side) -> Range<usize> {
    Range {
      start: self.diffs.first().map_or(0, |d| d.side(side).start),
      end: self.diffs.last().map_or(0, |d| d.side(side).end),
    }
  }

  pub fn lhs(&self) -> Range<usize> {
    self.side(Side::Lhs)
  }

  pub fn rhs(&self) -> Range<usize> {
    self.side(Side::Rhs)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diff::diff;
  use crate::tokens::split_lines;

  fn diff_lines(lhs: &[u8], rhs: &[u8]) -> Vec<DiffItem> {
    let lhs_lines: Vec<_> = split_lines(lhs);
    let rhs_lines: Vec<_> = split_lines(rhs);
    diff(&lhs_lines, &rhs_lines)
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
}
