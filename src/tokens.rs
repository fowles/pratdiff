use unicode_segmentation::UnicodeSegmentation;

/// Split `content` into lines, stripping line endings (`\r\n`, `\n`, `\r`).
pub fn split_lines(content: &[u8]) -> Vec<&[u8]> {
  if content.is_empty() {
    return vec![];
  }
  let content = content
    .strip_suffix(b"\r\n")
    .or_else(|| content.strip_suffix(b"\n"))
    .or_else(|| content.strip_suffix(b"\r"))
    .unwrap_or(content);

  // everyone knows that 80 is the one true line length.
  let mut lines = Vec::with_capacity(content.len() / 80);
  let mut start = 0;
  let mut i = 0;
  while i < content.len() {
    match (content.get(i), content.get(i + 1)) {
      (Some(b'\r'), Some(b'\n')) => {
        lines.push(&content[start..i]);
        i += 2;
        start = i;
      }
      (Some(b'\r') | Some(b'\n'), _) => {
        lines.push(&content[start..i]);
        i += 1;
        start = i;
      }
      _ => {
        i += 1;
      }
    }
  }
  lines.push(&content[start..]);
  lines
}

pub fn is_whitespace_token(token: &[u8]) -> bool {
  !token.is_empty() && token.iter().all(|b| (*b as char).is_ascii_whitespace())
}

pub fn tokenize_lines<'a>(lines: &[&'a [u8]]) -> Vec<&'a [u8]> {
  let mut v: Vec<_> = lines
    .iter()
    .flat_map(|l| ByteTokenIter::new(l).chain([b"\n" as &[u8]]))
    .collect();
  v.pop();
  v
}

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
    Err(e) => unsafe {
      std::str::from_utf8_unchecked(&content[..e.valid_up_to()])
    },
  }
}

impl<'a> Iterator for ByteTokenIter<'a> {
  type Item = &'a [u8];

  fn next(&mut self) -> Option<&'a [u8]> {
    if self.content.is_empty() {
      return None;
    }

    // If the valid prefix is exhausted we're sitting on an invalid sequence.
    // Emit it as a single opaque token, then compute the next valid prefix.
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
        if g.chars().next().is_some_and(pred) {
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
      first.len()
    };

    let (token, rest) = self.content.split_at(end);
    self.content = rest;
    self.valid = &self.valid[end..];
    Some(token)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn split_lines_lf() {
    assert_eq!(split_lines(b"a\nb\nc\n"), &[b"a", b"b", b"c"]);
    assert_eq!(split_lines(b"a\nb\nc"), &[b"a", b"b", b"c"]);
  }

  #[test]
  fn split_lines_crlf() {
    assert_eq!(split_lines(b"a\r\nb\r\nc\r\n"), &[b"a", b"b", b"c"]);
    assert_eq!(split_lines(b"a\r\nb\r\nc"), &[b"a", b"b", b"c"]);
  }

  #[test]
  fn split_lines_cr() {
    assert_eq!(split_lines(b"a\rb\rc\r"), &[b"a", b"b", b"c"]);
    assert_eq!(split_lines(b"a\rb\rc"), &[b"a", b"b", b"c"]);
  }

  #[test]
  fn split_lines_mixed() {
    assert_eq!(split_lines(b"a\r\nb\nc\rd"), &[b"a", b"b", b"c", b"d"]);
  }

  #[test]
  fn tokenize_invalid_utf8() {
    assert_eq!(
      tokenize_lines(&[b"foo\xffbar"]),
      &[b"foo" as &[u8], b"\xff", b"bar"],
    );
  }

  #[test]
  fn tokenize_combining_diacritics() {
    // "é" can be written as a unicode codepoint or a combining diacritic,
    // make sure we don't split the token before a combining diacritic.
    assert_eq!(tokenize_lines(&[b"cafe\xcc\x81"]), &[b"cafe\xcc\x81" as &[u8]],);
    assert_eq!(
      tokenize_lines(&[b"caf\xc3\xae"]),
      &[b"caf" as &[u8], b"\xc3\xae" as &[u8]],
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
