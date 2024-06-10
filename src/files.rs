use std::cmp::Ordering;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

use crate::printer::Printer;

pub fn diff(
  p: &mut Printer,
  lhs: &Path,
  rhs: &Path,
) -> Result<(), Box<dyn Error>> {
  match (lhs.metadata()?.is_dir(), rhs.metadata()?.is_dir()) {
    (false, false) => diff_file_candidates(p, Some(lhs), Some(rhs)),
    (true, true) => diff_directories(p, lhs, rhs),
    _ => Ok(p.print_directory_mismatch(lhs, rhs)?),
  }
}

fn diff_directories(
  p: &mut Printer,
  lhs: &Path,
  rhs: &Path,
) -> Result<(), Box<dyn Error>> {
  walk_dirs(lhs, rhs, |l, r| diff_entries(p, l, r))
}

fn diff_entries(
  p: &mut Printer,
  lhs: &Option<DirEntry>,
  rhs: &Option<DirEntry>,
) -> Result<(), Box<dyn Error>> {
  match (lhs, rhs) {
    (None, None) => Ok(()),
    (Some(lhs), None) => {
      if lhs.metadata()?.is_dir() {
        Ok(())
      } else {
        diff_file_candidates(p, Some(lhs.path()), None)
      }
    }
    (None, Some(rhs)) => {
      if rhs.metadata()?.is_dir() {
        Ok(())
      } else {
        diff_file_candidates(p, None, Some(rhs.path()))
      }
    }
    (Some(lhs), Some(rhs)) => {
      cfg_if::cfg_if! {
        if #[cfg(unix)] {
          use walkdir::DirEntryExt;
          if lhs.ino() == rhs.ino() {
            return Ok(())
          }
        }
      }

      match (lhs.metadata()?.is_dir(), rhs.metadata()?.is_dir()) {
        (false, false) => {
          diff_file_candidates(p, Some(lhs.path()), Some(rhs.path()))
        }
        (true, true) => Ok(()),
        _ => Ok(p.print_directory_mismatch(lhs.path(), rhs.path())?),
      }
    }
  }
}

fn diff_file_candidates(
  p: &mut Printer,
  lhs_path: Option<&Path>,
  rhs_path: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
  let lhs_raw = read(lhs_path)?;
  let rhs_raw = read(rhs_path)?;
  if lhs_raw == rhs_raw {
    return Ok(());
  }

  let (Ok(l), Ok(r)) = (String::from_utf8(lhs_raw), String::from_utf8(rhs_raw))
  else {
    p.print_binary_files_differ(lhs_path, rhs_path)?;
    return Ok(());
  };

  p.print_file_header(lhs_path, rhs_path)?;
  p.print_diff(&l, &r)?;
  Ok(())
}

fn walk_dirs<
  Handler: FnMut(&Option<DirEntry>, &Option<DirEntry>) -> Result<(), Box<dyn Error>>,
>(
  lhs_root: &Path,
  rhs_root: &Path,
  mut handler: Handler,
) -> Result<(), Box<dyn Error>>
where
{
  let mut lhs = WalkDir::new(lhs_root)
    .sort_by_file_name()
    .min_depth(1)
    .into_iter()
    .filter_map(|e| e.ok());
  let mut rhs = WalkDir::new(rhs_root)
    .sort_by_file_name()
    .min_depth(1)
    .into_iter()
    .filter_map(|e| e.ok());

  let compare = |l: &Option<DirEntry>, r: &Option<DirEntry>| match (&l, &r) {
    (None, None) => Ordering::Equal,
    (Some(_), None) => Ordering::Less,
    (None, Some(_)) => Ordering::Greater,
    (Some(lhs), Some(rhs)) => {
      let lhs_relative = lhs.path().strip_prefix(lhs_root).unwrap();
      let rhs_relative = rhs.path().strip_prefix(rhs_root).unwrap();
      lhs_relative.cmp(rhs_relative)
    }
  };
  let mut lhs_next = lhs.next();
  let mut rhs_next = rhs.next();
  loop {
    match compare(&lhs_next, &rhs_next) {
      Ordering::Equal => {
        if let (None, None) = (&lhs_next, &rhs_next) {
          return Ok(());
        }
        handler(&lhs_next, &rhs_next)?;
        lhs_next = lhs.next();
        rhs_next = rhs.next();
      }
      Ordering::Less => {
        handler(&lhs_next, &None)?;
        lhs_next = lhs.next();
      }
      Ordering::Greater => {
        handler(&None, &rhs_next)?;
        rhs_next = rhs.next();
      }
    }
  }
}

fn open(path: &Path) -> Result<Box<dyn Read>, Box<dyn Error>> {
  if path == Path::new("-") {
    Ok(Box::new(std::io::stdin()))
  } else {
    Ok(Box::new(File::open(path)?))
  }
}

fn read(path: Option<&Path>) -> Result<Vec<u8>, Box<dyn Error>> {
  let mut buffer = Vec::new();
  if let Some(path) = path {
    let mut file = open(path)?;
    file.read_to_end(&mut buffer)?;
  }
  Ok(buffer)
}

#[cfg(test)]
mod tests {
  fn filename(entry: &Option<DirEntry>) -> Option<String> {
    entry
      .as_ref()
      .and_then(|e| e.path().file_name())
      .and_then(|f| f.to_str())
      .map(|s| s.to_owned())
  }

  use super::*;

  #[test]
  fn directories() -> Result<(), Box<dyn Error>> {
    let old = tempfile::tempdir()?;
    File::create(old.path().join("1"))?;
    File::create(old.path().join("2"))?;
    File::create(old.path().join("3"))?;
    let new = tempfile::tempdir()?;
    File::create(new.path().join("1"))?;
    File::create(new.path().join("3"))?;
    File::create(new.path().join("4"))?;

    let mut v = Vec::<(Option<String>, Option<String>)>::new();
    walk_dirs(old.path(), new.path(), |lhs, rhs| {
      v.push((filename(&lhs), filename(&rhs)));
      Ok(())
    })?;
    assert_eq!(
      v,
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
