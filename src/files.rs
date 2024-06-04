use either::Either;
use either::Either::*;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Stdin;

pub enum Contents {
  Text(String),
  Binary(Vec<u8>),
}
impl Contents {
  pub fn as_bytes(&self) -> &[u8] {
    match self {
      Contents::Text(s) => &s.as_bytes(),
      Contents::Binary(v) => &v,
    }
  }
}

fn open(path: &str) -> Result<Either<Stdin, File>, Box<dyn Error>> {
  if path == "-" {
    Ok(Left(std::io::stdin()))
  } else {
    Ok(Right(File::open(path)?))
  }
}

pub fn read(path: &str) -> Result<Contents, Box<dyn Error>> {
  let mut file = open(path)?;

  let mut buffer = Vec::new();
  file.read_to_end(&mut buffer)?;

  if let Ok(_) = std::str::from_utf8(&buffer) {
    unsafe { Ok(Contents::Text(String::from_utf8_unchecked(buffer))) }
  } else {
    Ok(Contents::Binary(buffer))
  }
}
