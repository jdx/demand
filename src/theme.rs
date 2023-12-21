use termcolor::{Color, ColorSpec};

/// Theme for styling the UI.
pub struct Theme {
    pub title: ColorSpec,
    pub error_indicator: ColorSpec,
    pub description: ColorSpec,
    pub cursor: ColorSpec,
    pub selected_option: ColorSpec,
    pub selected_prefix: String,
    pub selected_prefix_fg: ColorSpec,
    pub unselected_option: ColorSpec,
    pub unselected_prefix: String,
    pub unselected_prefix_fg: ColorSpec,

    pub input_cursor: ColorSpec,
    // input_placeholder: ColorSpec,
    // input_prompt: ColorSpec,
    pub help_key: ColorSpec,
    pub help_desc: ColorSpec,
    pub help_sep: ColorSpec,
}

impl Theme {
    /// Create a new theme with no colors.
    pub fn new() -> Self {
        Self {
            title: ColorSpec::new(),
            error_indicator: ColorSpec::new(),
            description: ColorSpec::new(),
            cursor: ColorSpec::new(),
            selected_prefix: String::from(" x"),
            selected_prefix_fg: ColorSpec::new(),
            selected_option: ColorSpec::new(),
            unselected_prefix: String::from("  "),
            unselected_prefix_fg: ColorSpec::new(),
            unselected_option: ColorSpec::new(),
            input_cursor: ColorSpec::new(),
            help_key: ColorSpec::new(),
            help_desc: ColorSpec::new(),
            help_sep: ColorSpec::new(),
        }
    }

    /// Create a new theme with the default colors.
    pub fn charm() -> Self {
        let normal = Color::Ansi256(252);
        let indigo = Color::Rgb(117, 113, 249);
        let red = Color::Rgb(255, 70, 114);
        let fuchsia = Color::Rgb(247, 128, 226);
        let green = Color::Rgb(2, 191, 135);

        let mut title = make_color(indigo);
        title.set_bold(true);

        Self {
            title,
            error_indicator: make_color(red),
            description: make_color(Color::Ansi256(243)),
            cursor: make_color(fuchsia),
            selected_prefix: String::from(" ✓"),
            selected_prefix_fg: make_color(Color::Rgb(2, 168, 119)),
            selected_option: make_color(green),
            unselected_prefix: String::from(" •"),
            unselected_prefix_fg: make_color(Color::Ansi256(243)),
            unselected_option: make_color(normal),

            input_cursor: make_color(green),
            // input_placeholder: make_color(Color::Ansi256(238)),
            // input_prompt: make_color(fuchsia),
            help_key: make_color(Color::Rgb(98, 98, 98)),
            help_desc: make_color(Color::Rgb(74, 74, 74)),
            help_sep: make_color(Color::Rgb(60, 60, 60)),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        if console::colors_enabled_stderr() {
            Theme::charm()
        } else {
            Theme::new()
        }
    }
}

fn make_color(color: Color) -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(color));
    spec
}
