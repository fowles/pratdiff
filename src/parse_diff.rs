use std::borrow::Cow;

use crate::tokens::split_lines;

/// Preamble lines (git `diff --git`, `index`, etc.) plus `---`/`+++` headers
/// and the hunks that follow, for one file in the input diff.
#[derive(Debug, Default, PartialEq)]
pub struct ParsedFileDiff<'a> {
  /// Lines before `---`/`+++`: git preamble, etc. Emitted verbatim.
  pub preamble: Vec<&'a [u8]>,
  /// Path extracted from the `--- ...` line, if present.
  pub old_path: Option<std::path::PathBuf>,
  /// Path extracted from the `+++ ...` line, if present.
  pub new_path: Option<std::path::PathBuf>,
  pub hunks: Vec<ParsedHunk<'a>>,
}

#[derive(Debug, Default, PartialEq)]
pub struct ParsedHunk<'a> {
  /// The `@@ ... @@` hunk header in unified format.
  /// Borrowed from the input for unified hunks; owned for traditional hunks
  /// whose header is synthesized from line-range numbers.
  pub header: Option<Cow<'a, [u8]>>,
  pub items: Vec<ParsedHunkItem<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum ParsedHunkItem<'a> {
  /// A single context line with the leader stripped.
  Context(&'a [u8]),
  Mutation(ParsedMutation<'a>),
}

/// Old and new lines with leader stripped.
#[derive(Debug, PartialEq)]
pub struct ParsedMutation<'a> {
  pub old: Vec<&'a [u8]>,
  pub new: Vec<&'a [u8]>,
}

pub type ParsedDiff<'a> = Vec<ParsedFileDiff<'a>>;

/// Parse a diff from pre-ANSI-stripped bytes, returning a structure that
/// borrows slices directly from `input`.
pub fn parse(input: &[u8]) -> ParsedDiff<'_> {
  Parser::new().run(&split_lines(input))
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum State {
  BeforeHunk,
  InHunk,
}

struct Parser<'a> {
  result: ParsedDiff<'a>,
  current_file: ParsedFileDiff<'a>,
  current_hunk: Option<ParsedHunk<'a>>,
  pending_old: Vec<&'a [u8]>,
  pending_new: Vec<&'a [u8]>,
  state: State,
}

impl<'a> Parser<'a> {
  fn new() -> Self {
    Parser {
      result: vec![],
      current_file: ParsedFileDiff::default(),
      current_hunk: None,
      pending_old: vec![],
      pending_new: vec![],
      state: State::BeforeHunk,
    }
  }

  fn run(mut self, lines: &[&'a [u8]]) -> ParsedDiff<'a> {
    for &line in lines {
      self.process_line(line);
    }
    self.close_file();
    self.result
  }

  fn process_line(&mut self, line: &'a [u8]) {
    match self.state {
      State::BeforeHunk => self.before_hunk_line(line),
      State::InHunk => self.hunk_line(line),
    }
  }

  fn before_hunk_line(&mut self, line: &'a [u8]) {
    if line.starts_with(b"diff --git ") {
      self.push_current_file();
      self.current_file.preamble.push(line);
    } else if let Some(path) = try_parse_file_header(b"--- ", line) {
      self.current_file.old_path = Some(path);
    } else if let Some(path) = try_parse_file_header(b"+++ ", line) {
      self.current_file.new_path = Some(path);
    } else if line.starts_with(b"@@ ") {
      self.start_hunk(Cow::Borrowed(line));
    } else if let Some(header) = try_parse_traditional_hunk_header(line) {
      self.start_hunk(Cow::Owned(header));
    } else {
      self.current_file.preamble.push(line);
    }
  }

  fn hunk_line(&mut self, line: &'a [u8]) {
    if line.starts_with(b"diff --git ") {
      self.close_file();
      self.current_file.preamble.push(line);
    } else if let Some(path) = try_parse_file_header(b"--- ", line) {
      self.close_file();
      self.current_file.old_path = Some(path);
    } else if line.starts_with(b"@@ ") {
      self.close_hunk();
      self.start_hunk(Cow::Borrowed(line));
    } else if let Some(header) = try_parse_traditional_hunk_header(line) {
      self.close_hunk();
      self.start_hunk(Cow::Owned(header));
    } else if line == b"---" {
      // separator between `<` and `>` lines in traditional diffs.
    } else if let Some(rest) = line.strip_prefix(b"-") {
      self.pending_old.push(rest);
    } else if let Some(rest) = line.strip_prefix(b"+") {
      self.pending_new.push(rest);
    } else if let Some(rest) = line.strip_prefix(b"< ") {
      self.pending_old.push(rest);
    } else if let Some(rest) = line.strip_prefix(b"> ") {
      self.pending_new.push(rest);
    } else {
      self.flush_mutation();
      self.push_context(line);
    }
  }

  fn push_context(&mut self, line: &'a [u8]) {
    let content = if line.starts_with(b" ") { &line[1..] } else { line };
    self
      .current_hunk
      .as_mut()
      .unwrap()
      .items
      .push(ParsedHunkItem::Context(content));
  }

  fn start_hunk(&mut self, header: Cow<'a, [u8]>) {
    self.current_hunk =
      Some(ParsedHunk { header: Some(header), items: vec![] });
    self.state = State::InHunk;
  }

  fn close_hunk(&mut self) {
    self.flush_mutation();
    self.push_current_hunk();
  }

  fn close_file(&mut self) {
    self.close_hunk();
    self.push_current_file();
    self.state = State::BeforeHunk;
  }

  fn flush_mutation(&mut self) {
    if self.pending_old.is_empty() && self.pending_new.is_empty() {
      return;
    }
    let m = ParsedMutation {
      old: std::mem::take(&mut self.pending_old),
      new: std::mem::take(&mut self.pending_new),
    };
    if let Some(hunk) = self.current_hunk.as_mut() {
      hunk.items.push(ParsedHunkItem::Mutation(m));
    }
  }

  fn push_current_hunk(&mut self) {
    if let Some(hunk) = self.current_hunk.take()
      && (!hunk.items.is_empty() || hunk.header.is_some())
    {
      self.current_file.hunks.push(hunk);
    }
  }

  fn push_current_file(&mut self) {
    let file = std::mem::take(&mut self.current_file);
    if !file.preamble.is_empty()
      || file.old_path.is_some()
      || file.new_path.is_some()
      || !file.hunks.is_empty()
    {
      self.result.push(file);
    }
  }
}

fn try_parse_file_header(
  prefix: &[u8],
  line: &[u8],
) -> Option<std::path::PathBuf> {
  let s = line.strip_prefix(prefix)?;
  let s = s
    .strip_prefix(b"a/")
    .or_else(|| s.strip_prefix(b"b/"))
    .unwrap_or(s);
  Some(std::path::PathBuf::from(String::from_utf8_lossy(s).as_ref()))
}

/// If `line` is a traditional diff range header (e.g. `3c3`, `1,4d2`,
/// `2a3,5`), return the equivalent unified `@@ ... @@` header bytes.
fn try_parse_traditional_hunk_header(line: &[u8]) -> Option<Vec<u8>> {
  use std::sync::OnceLock;

  use regex::bytes::Regex;
  static RE: OnceLock<Regex> = OnceLock::new();
  let re = RE.get_or_init(|| {
    Regex::new(r"^(\d+)(?:,(\d+))?([acd])(\d+)(?:,(\d+))?$").unwrap()
  });
  let caps = re.captures(line)?;
  let num = |i| -> u64 {
    std::str::from_utf8(caps.get(i).unwrap().as_bytes())
      .unwrap()
      .parse()
      .unwrap()
  };
  let l1 = num(1);
  let l2 = caps.get(2).map(|_| num(2)).unwrap_or(l1);
  let cmd = caps.get(3).unwrap().as_bytes()[0];
  let r1 = num(4);
  let r2 = caps.get(5).map(|_| num(5)).unwrap_or(r1);
  let (ls, lc, rs, rc) = match cmd {
    b'd' => (l1, l2 - l1 + 1, r1 + 1, 0),
    b'a' => (l1 + 1, 0, r1, r2 - r1 + 1),
    b'c' => (l1, l2 - l1 + 1, r1, r2 - r1 + 1),
    _ => return None,
  };
  Some(format!("@@ -{},{} +{},{} @@", ls, lc, rs, rc).into_bytes())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  fn mutation_counts(hunk: &ParsedHunk) -> (usize, usize) {
    let mut old = 0;
    let mut new = 0;
    for item in &hunk.items {
      if let ParsedHunkItem::Mutation(m) = item {
        old += m.old.len();
        new += m.new.len();
      }
    }
    (old, new)
  }

  fn context_count(hunk: &ParsedHunk) -> usize {
    hunk
      .items
      .iter()
      .filter(|i| matches!(i, ParsedHunkItem::Context(_)))
      .count()
  }

  fn mutation_count(hunk: &ParsedHunk) -> usize {
    hunk
      .items
      .iter()
      .filter(|i| matches!(i, ParsedHunkItem::Mutation(_)))
      .count()
  }

  #[test]
  fn parse_empty() {
    assert!(parse(b"").is_empty());
  }

  #[test]
  fn parse_unified_single_hunk() {
    let input = b"\
--- a/file.txt\n\
+++ b/file.txt\n\
@@ -1,2 +1,2 @@\n\
-old line\n\
+new line\n\
";
    let diffs = parse(input);
    assert_eq!(diffs.len(), 1);
    let f = &diffs[0];
    assert_eq!(f.hunks.len(), 1);
    let h = &f.hunks[0];
    assert!(h.header.is_some());
    assert_eq!(mutation_count(h), 1);
    let (old, new) = mutation_counts(h);
    assert_eq!(old, 1);
    assert_eq!(new, 1);
  }

  #[test]
  fn parse_unified_context_lines() {
    let input = b"\
--- a/file.txt\n\
+++ b/file.txt\n\
@@ -1,5 +1,5 @@\n\
 ctx before\n\
-old\n\
+new\n\
 ctx after\n\
";
    let diffs = parse(input);
    let h = &diffs[0].hunks[0];
    assert_eq!(context_count(h), 2);
    assert_eq!(mutation_count(h), 1);
  }

  #[test]
  fn parse_git_headers() {
    let input = b"\
diff --git a/file.txt b/file.txt\n\
index abc1234..def5678 100644\n\
--- a/file.txt\n\
+++ b/file.txt\n\
@@ -1 +1 @@\n\
-old\n\
+new\n\
";
    let diffs = parse(input);
    assert_eq!(diffs.len(), 1);
    let f = &diffs[0];
    // "diff --git" and "index" lines are in preamble.
    assert_eq!(f.preamble.len(), 2);
    assert!(f.preamble[0].starts_with(b"diff --git"));
    assert!(f.preamble[1].starts_with(b"index"));
    assert!(f.old_path.is_some());
    assert!(f.new_path.is_some());
  }

  #[test]
  fn parse_traditional_change() {
    let input = b"\
3c3\n\
< old line\n\
---\n\
> new line\n\
";
    let diffs = parse(input);
    assert_eq!(diffs.len(), 1);
    let h = &diffs[0].hunks[0];
    assert_eq!(
      h.header.as_deref(),
      Some(b"@@ -3,1 +3,1 @@" as &[u8]),
      "traditional 3c3 should produce unified header"
    );
    let (old, new) = mutation_counts(h);
    assert_eq!(old, 1);
    assert_eq!(new, 1);
  }

  #[test]
  fn parse_traditional_add() {
    let input = b"\
3a4\n\
> added line\n\
";
    let diffs = parse(input);
    let h = &diffs[0].hunks[0];
    let (old, new) = mutation_counts(h);
    assert_eq!(old, 0);
    assert_eq!(new, 1);
  }

  #[test]
  fn parse_traditional_delete() {
    let input = b"\
1,3d2\n\
< line 1\n\
< line 2\n\
< line 3\n\
";
    let diffs = parse(input);
    let h = &diffs[0].hunks[0];
    let (old, new) = mutation_counts(h);
    assert_eq!(old, 3);
    assert_eq!(new, 0);
  }

  #[test]
  fn parse_multifile() {
    let input = b"\
--- a/foo.txt\n\
+++ b/foo.txt\n\
@@ -1 +1 @@\n\
-old foo\n\
+new foo\n\
--- a/bar.txt\n\
+++ b/bar.txt\n\
@@ -1 +1 @@\n\
-old bar\n\
+new bar\n\
";
    let diffs = parse(input);
    assert_eq!(diffs.len(), 2);
  }

  #[test]
  fn try_parse_traditional_hunk_header_patterns() {
    assert!(try_parse_traditional_hunk_header(b"3c3").is_some());
    assert!(try_parse_traditional_hunk_header(b"1,4d2").is_some());
    assert!(try_parse_traditional_hunk_header(b"2a3,5").is_some());
    assert!(try_parse_traditional_hunk_header(b"10,20c15,25").is_some());
    assert!(try_parse_traditional_hunk_header(b"@@ -1 +1 @@").is_none());
    assert!(try_parse_traditional_hunk_header(b"--- a/file").is_none());
    assert!(try_parse_traditional_hunk_header(b"").is_none());
    assert!(try_parse_traditional_hunk_header(b"abc").is_none());
  }
}
