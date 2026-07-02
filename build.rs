use vergen::Emitter;
use vergen::Build;
use vergen_gitcl::Gitcl;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let build = Build::builder().build_date(true).build();
  let gitcl = Gitcl::builder().describe(true, false, None).build();
  Emitter::default()
    .add_instructions(&build)?
    .add_instructions(&gitcl)?
    .emit()?;
  Ok(())
}
