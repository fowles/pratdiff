use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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

fn open(path: &Path) -> Result<Box<dyn Read>, Box<dyn Error>> {
  if path == Path::new("-") {
    Ok(Box::new(std::io::stdin()))
  } else {
    Ok(Box::new(File::open(path)?))
  }
}

pub fn read(path: &Path) -> Result<Contents, Box<dyn Error>> {
  let mut file = open(path)?;

  let mut buffer = Vec::new();
  file.read_to_end(&mut buffer)?;

  if let Ok(_) = std::str::from_utf8(&buffer) {
    unsafe { Ok(Contents::Text(String::from_utf8_unchecked(buffer))) }
  } else {
    Ok(Contents::Binary(buffer))
  }
}
