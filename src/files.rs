#![allow(unused)] // TODO(kfm): remove this

use std::cmp::Ordering;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

use crate::printer::Printer;

pub fn diff_files(
  p: &mut Printer,
  lhs_path: &Path,
  rhs_path: &Path,
) -> Result<(), Box<dyn Error>> {
  let lhs_raw = read(lhs_path)?;
  let rhs_raw = read(rhs_path)?;
  if lhs_raw == rhs_raw {
    return Ok(());
  }

  let (Ok(l), Ok(r)) = (String::from_utf8(lhs_raw), String::from_utf8(rhs_raw))
  else {
    p.print_binary_files_differ(&lhs_path.display(), &rhs_path.display());
    return Ok(());
  };

  p.print_file_header(&lhs_path.display(), &rhs_path.display());
  p.print_diff(&l, &r);
  Ok(())
}

fn walk_dirs<Handler: FnMut(Option<&Path>, Option<&Path>)>(
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
    (Some(lhs), None) => Ordering::Less,
    (None, Some(rhs)) => Ordering::Greater,
    (Some(lhs), Some(rhs)) => {
      let lhs_relative = lhs.path().strip_prefix(lhs_root).unwrap();
      let rhs_relative = rhs.path().strip_prefix(rhs_root).unwrap();
      lhs_relative.cmp(rhs_relative)
    }
  };
  let mut lhs_next = lhs.next();
  let mut rhs_next = rhs.next();
  loop {
    match (compare(&lhs_next, &rhs_next)) {
      Ordering::Equal => {
        if let (None, None) = (&lhs_next, &rhs_next) {
          return Ok(());
        }
        handler(Some(lhs_next.unwrap().path()), Some(rhs_next.unwrap().path()));
        lhs_next = lhs.next();
        rhs_next = rhs.next();
      }
      Ordering::Less => {
        handler(Some(lhs_next.unwrap().path()), None);
        lhs_next = lhs.next();
      }
      Ordering::Greater => {
        handler(None, Some(rhs_next.unwrap().path()));
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

fn read(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
  let mut file = open(path)?;

  let mut buffer = Vec::new();
  file.read_to_end(&mut buffer)?;
  Ok(buffer)
}

fn filename(path: &Option<&Path>) -> Option<String> {
  path
    .and_then(|f| f.file_name())
    .and_then(|f| f.to_str())
    .map(|s| s.to_owned())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn directories() -> Result<(), Box<dyn Error>> {
    let old = tempfile::tempdir()?;
    File::create(old.path().join("1"));
    File::create(old.path().join("2"));
    File::create(old.path().join("3"));
    let new = tempfile::tempdir()?;
    File::create(new.path().join("1"));
    File::create(new.path().join("3"));
    File::create(new.path().join("4"));

    let mut v = Vec::<(Option<String>, Option<String>)>::new();
    walk_dirs(old.path(), new.path(), |lhs, rhs| {
      v.push((filename(&lhs), filename(&rhs)));
    });
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
