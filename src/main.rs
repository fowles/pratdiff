use anstream::println;
use clap::Parser;
use owo_colors::OwoColorize;

#[derive(Parser, Debug)]
#[command(version)]
#[command(about = "Diff files using patience algorithm")]
struct Args {
  /// Path to old file or `-` for stdin.
  #[clap(name = "OLD_FILE")]
  lhs: String,

  /// Path to new file or `-` for stdin.
  #[clap(name = "NEW_FILE")]
  rhs: String,
}

fn main() {
  let _ = Args::parse();

  println!("{}", "Hello, world!".red());
}
