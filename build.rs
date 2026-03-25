use vergen::{BuildBuilder, Emitter};
use vergen_gitcl::GitclBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let build = BuildBuilder::default().build_date(true).build()?;
  let gitcl = GitclBuilder::default().describe(true, false, None).build()?;
  Emitter::default()
    .add_instructions(&build)?
    .add_instructions(&gitcl)?
    .emit()?;
  Ok(())
}
