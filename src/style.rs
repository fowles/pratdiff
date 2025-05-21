use owo_colors::Style;

/// Defaults to using the terminal default colors.
#[derive(Default)]
pub struct Styles {
  pub(crate) header: Style,
  pub(crate) separator: Style,
  pub(crate) both: Style,
  pub(crate) old: Style,
  pub(crate) old_dim: Style,
  pub(crate) new: Style,
  pub(crate) new_dim: Style,
}

impl Styles {
  /// A simple set of color choices reasonable for most colorized terminal output.
  pub fn simple() -> Styles {
    Styles {
      header: Style::new().bold().white(),
      separator: Style::new().cyan(),
      both: Style::new().default_color(),
      old: Style::new().red(),
      new: Style::new().green(),
      old_dim: Style::new().dimmed(),
      new_dim: Style::new().default_color(),
    }
  }
}
