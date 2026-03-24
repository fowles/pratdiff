use vergen::EmitBuilder;

fn main() {
  EmitBuilder::builder()
    .build_date()
    .git_describe(true, false, None)
    .emit()
    .unwrap();
}
