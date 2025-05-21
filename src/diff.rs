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
        if hunk.diffs.len() <= 1 && matches!(hunk.diffs[0], Match { .. }) {
          return None;
        }

        if context == 0 {
          hunk.diffs.retain(|d| matches!(d, Mutation { .. }));
          return Some(hunk);
        }

        if let Some(Match { lhs, rhs }) = hunk.diffs.first_mut() {
          if lhs.len() > context {
            lhs.start = lhs.end - context;
            rhs.start = rhs.end - context;
          }
        }
        if let Some(Match { lhs, rhs }) = hunk.diffs.last_mut() {
          if lhs.len() > context {
            lhs.end = lhs.start + context;
            rhs.end = rhs.start + context;
          }
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Diffs {
  pub(crate) vec: Vec<DiffItem>,
}

impl Diffs {
  pub(crate) fn add_match(&mut self, len: usize) {
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

  pub(crate) fn add_mutation(&mut self, lhs: usize, rhs: usize) {
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
