use owo_colors::Style;

/// Defaults to using the terminal default colors.
#[derive(Default)]
pub struct Styles {
  pub header: Style,
  pub separator: Style,
  pub both: Style,
  pub old: Style,
  pub old_dim: Style,
  pub new: Style,
  pub new_dim: Style,
}

impl Styles {
  /// A simple set of color choices reasonable for most colorized terminal
  /// output.
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
