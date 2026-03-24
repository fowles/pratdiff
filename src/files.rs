use std::cmp::Ordering;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use walkdir::DirEntry;
use walkdir::WalkDir;

/// An event produced by walking a pair of paths.
pub enum FilePairEvent {
  /// A pair of diffable (non-identical, non-binary) text files.
  TextDiff {
    lhs_path: Option<PathBuf>,
    rhs_path: Option<PathBuf>,
    lhs_content: Vec<u8>,
    rhs_content: Vec<u8>,
  },
  /// Files that differ but at least one is non-UTF-8.
  Binary {
    lhs_path: Option<PathBuf>,
    rhs_path: Option<PathBuf>,
  },
  /// One path is a file and the other is a directory.
  TypeMismatch { lhs_path: PathBuf, rhs_path: PathBuf },
  /// An I/O or other error while processing this pair.
  IoError {
    lhs_path: Option<PathBuf>,
    rhs_path: Option<PathBuf>,
    err: String,
  },
}

enum IterState {
  /// Walking two directory trees in parallel.
  Dirs(DirWalkState),
  /// A single pre-computed event.
  Once(Option<FilePairEvent>),
}

struct DirWalkState {
  lhs_root: PathBuf,
  rhs_root: PathBuf,
  lhs_iter: Box<dyn Iterator<Item = DirEntry>>,
  rhs_iter: Box<dyn Iterator<Item = DirEntry>>,
  lhs_next: Option<DirEntry>,
  rhs_next: Option<DirEntry>,
}

fn make_walk_iter(root: &Path) -> Box<dyn Iterator<Item = DirEntry>> {
  Box::new(
    WalkDir::new(root)
      .sort_by_file_name()
      .min_depth(1)
      .into_iter()
      .filter_map(|e| e.ok()),
  )
}

impl DirWalkState {
  /// Advance the walk, skipping identical things.
  fn advance(&mut self) -> Option<FilePairEvent> {
    loop {
      let ord = compare_entries(
        &self.lhs_next,
        &self.rhs_next,
        &self.lhs_root,
        &self.rhs_root,
      );
      match ord {
        Ordering::Equal
          if self.lhs_next.is_none() && self.rhs_next.is_none() =>
        {
          return None;
        }
        Ordering::Equal => {
          let lhs = self.lhs_next.take();
          let rhs = self.rhs_next.take();
          self.lhs_next = self.lhs_iter.next();
          self.rhs_next = self.rhs_iter.next();
          if let Some(event) = process_entry_pair(lhs.as_ref(), rhs.as_ref()) {
            return Some(event);
          }
        }
        Ordering::Less => {
          let lhs = self.lhs_next.take();
          self.lhs_next = self.lhs_iter.next();
          if let Some(event) = process_entry_pair(lhs.as_ref(), None) {
            return Some(event);
          }
        }
        Ordering::Greater => {
          let rhs = self.rhs_next.take();
          self.rhs_next = self.rhs_iter.next();
          if let Some(event) = process_entry_pair(None, rhs.as_ref()) {
            return Some(event);
          }
        }
      }
    }
  }
}

/// A lazy iterator over file pair events.
pub struct FilePairIter {
  state: IterState,
}

impl Iterator for FilePairIter {
  type Item = FilePairEvent;

  fn next(&mut self) -> Option<FilePairEvent> {
    match &mut self.state {
      IterState::Dirs(walk) => walk.advance(),
      IterState::Once(event) => event.take(),
    }
  }
}

/// Walk lhs and rhs (files or directory trees) and yield an event for each
/// differing file pair encountered.
pub fn walk_file_pairs(lhs: &Path, rhs: &Path) -> FilePairIter {
  let stdin = Path::new("-");
  if lhs == stdin || rhs == stdin {
    return FilePairIter {
      state: IterState::Once(process_file_pair(
        Some(lhs.to_path_buf()),
        Some(rhs.to_path_buf()),
      )),
    };
  }
  let lhs = lhs.to_path_buf();
  let rhs = rhs.to_path_buf();

  let lhs_is_dir = match lhs.metadata() {
    Ok(m) => m.is_dir(),
    Err(e) => {
      return FilePairIter {
        state: IterState::Once(Some(FilePairEvent::IoError {
          lhs_path: Some(lhs),
          rhs_path: Some(rhs),
          err: e.to_string(),
        })),
      };
    }
  };
  let rhs_is_dir = match rhs.metadata() {
    Ok(m) => m.is_dir(),
    Err(e) => {
      return FilePairIter {
        state: IterState::Once(Some(FilePairEvent::IoError {
          lhs_path: Some(lhs),
          rhs_path: Some(rhs),
          err: e.to_string(),
        })),
      };
    }
  };
  match (lhs_is_dir, rhs_is_dir) {
    (false, false) => FilePairIter {
      state: IterState::Once(process_file_pair(Some(lhs), Some(rhs))),
    },
    (true, true) => {
      let lhs_root = lhs;
      let mut lhs_iter = make_walk_iter(&lhs_root);
      let lhs_next = lhs_iter.next();

      let rhs_root = rhs;
      let mut rhs_iter = make_walk_iter(&rhs_root);
      let rhs_next = rhs_iter.next();

      FilePairIter {
        state: IterState::Dirs(DirWalkState {
          lhs_root,
          rhs_root,
          lhs_iter,
          rhs_iter,
          lhs_next,
          rhs_next,
        }),
      }
    }
    _ => FilePairIter {
      state: IterState::Once(Some(FilePairEvent::TypeMismatch {
        lhs_path: lhs,
        rhs_path: rhs,
      })),
    },
  }
}

fn compare_entries(
  l: &Option<DirEntry>,
  r: &Option<DirEntry>,
  lhs_root: &Path,
  rhs_root: &Path,
) -> Ordering {
  match (l, r) {
    (None, None) => Ordering::Equal,
    (Some(_), None) => Ordering::Less,
    (None, Some(_)) => Ordering::Greater,
    (Some(lhs), Some(rhs)) => {
      let lhs_rel = lhs.path().strip_prefix(lhs_root).unwrap();
      let rhs_rel = rhs.path().strip_prefix(rhs_root).unwrap();
      lhs_rel.cmp(rhs_rel)
    }
  }
}

fn is_dir(entry: &DirEntry) -> bool {
  entry.metadata().map(|m| m.is_dir()).unwrap_or(false)
}

/// Process one matched directory entry pair. Returns `None` for pairs that
/// should be skipped (directories, identical inodes, identical file contents).
fn process_entry_pair(
  lhs: Option<&DirEntry>,
  rhs: Option<&DirEntry>,
) -> Option<FilePairEvent> {
  match (lhs, rhs) {
    (None, None) => None,
    (Some(lhs), None) => {
      if is_dir(lhs) {
        None
      } else {
        process_file_pair(Some(lhs.path().to_path_buf()), None)
      }
    }
    (None, Some(rhs)) => {
      if is_dir(rhs) {
        None
      } else {
        process_file_pair(None, Some(rhs.path().to_path_buf()))
      }
    }
    (Some(lhs), Some(rhs)) => {
      cfg_if::cfg_if! {
        if #[cfg(unix)] {
          use walkdir::DirEntryExt;
          if lhs.ino() == rhs.ino() {
            return None;
          }
        }
      }
      match (is_dir(lhs), is_dir(rhs)) {
        (true, true) => None,
        (false, false) => process_file_pair(
          Some(lhs.path().to_path_buf()),
          Some(rhs.path().to_path_buf()),
        ),
        _ => Some(FilePairEvent::TypeMismatch {
          lhs_path: lhs.path().to_path_buf(),
          rhs_path: rhs.path().to_path_buf(),
        }),
      }
    }
  }
}

/// Read a file pair, returning the appropriate event or `None` if identical.
fn process_file_pair(
  lhs_path: Option<PathBuf>,
  rhs_path: Option<PathBuf>,
) -> Option<FilePairEvent> {
  let lhs = read(lhs_path.as_deref());
  let rhs = read(rhs_path.as_deref());
  match (lhs, rhs) {
    (Err(e), _) | (_, Err(e)) => {
      Some(FilePairEvent::IoError { lhs_path, rhs_path, err: e.to_string() })
    }
    (Ok(lhs_content), Ok(rhs_content)) => {
      if lhs_content == rhs_content {
        return None;
      }
      if std::str::from_utf8(&lhs_content).is_err()
        || std::str::from_utf8(&rhs_content).is_err()
      {
        return Some(FilePairEvent::Binary { lhs_path, rhs_path });
      }
      Some(FilePairEvent::TextDiff {
        lhs_path,
        rhs_path,
        lhs_content,
        rhs_content,
      })
    }
  }
}

fn open(path: &Path) -> Result<Box<dyn Read>, Box<dyn std::error::Error>> {
  if path == Path::new("-") {
    Ok(Box::new(std::io::stdin()))
  } else {
    Ok(Box::new(File::open(path)?))
  }
}

fn read(path: Option<&Path>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
  let mut buffer = Vec::new();
  if let Some(path) = path {
    let mut file = open(path)?;
    file.read_to_end(&mut buffer)?;
  }
  Ok(buffer)
}

#[cfg(test)]
mod tests {
  use super::*;

  // Tests walk_dirs merge ordering using a lightweight callback (no file I/O).
  fn walk_dirs_order(
    lhs_root: &Path,
    rhs_root: &Path,
  ) -> Vec<(Option<String>, Option<String>)> {
    let mut lhs_iter = make_walk_iter(lhs_root);
    let mut lhs_next = lhs_iter.next();

    let mut rhs_iter = make_walk_iter(rhs_root);
    let mut rhs_next = rhs_iter.next();

    let mut result = Vec::new();
    loop {
      let ord = compare_entries(&lhs_next, &rhs_next, lhs_root, rhs_root);
      match ord {
        Ordering::Equal if lhs_next.is_none() && rhs_next.is_none() => break,
        Ordering::Equal => {
          result.push((filename(&lhs_next), filename(&rhs_next)));
          lhs_next = lhs_iter.next();
          rhs_next = rhs_iter.next();
        }
        Ordering::Less => {
          result.push((filename(&lhs_next), None));
          lhs_next = lhs_iter.next();
        }
        Ordering::Greater => {
          result.push((None, filename(&rhs_next)));
          rhs_next = rhs_iter.next();
        }
      }
    }
    result
  }

  fn filename(entry: &Option<DirEntry>) -> Option<String> {
    entry
      .as_ref()
      .and_then(|e| e.path().file_name())
      .and_then(|f| f.to_str())
      .map(|s| s.to_owned())
  }

  #[test]
  fn directories() -> Result<(), Box<dyn std::error::Error>> {
    let old = tempfile::tempdir()?;
    File::create(old.path().join("1"))?;
    File::create(old.path().join("2"))?;
    File::create(old.path().join("3"))?;
    let new = tempfile::tempdir()?;
    File::create(new.path().join("1"))?;
    File::create(new.path().join("3"))?;
    File::create(new.path().join("4"))?;

    assert_eq!(
      walk_dirs_order(old.path(), new.path()),
      &[
        (Some("1".to_owned()), Some("1".to_owned())),
        (Some("2".to_owned()), None),
        (Some("3".to_owned()), Some("3".to_owned())),
        (None, Some("4".to_owned())),
      ]
    );
    Ok(())
  }
}
