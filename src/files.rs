#![allow(unused)] // TODO(kfm): remove this

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

use crate::printer::Printer;

pub fn diff(
  p: &mut Printer,
  lhs: &Path,
  rhs: &Path,
) -> Result<(), Box<dyn Error>> {

  Ok(())
}

pub fn diff_files(
  p: &mut Printer,
  lhs_path: &Path,
  rhs_path: &Path,
) -> Result<(), Box<dyn Error>> {
  let lhs_raw = read(&lhs_path)?;
  let rhs_raw = read(&rhs_path)?;
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
