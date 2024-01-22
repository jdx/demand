use once_cell::sync::Lazy;
use termcolor::{Color, ColorSpec};

pub(crate) static DEFAULT: Lazy<Theme> = Lazy::new(Theme::default);

/// Theme for styling the UI.
///
/// # Example
///
/// ```
/// use demand::Theme;
///
/// let custom_theme = Theme {
///   selected_prefix: String::from(" •"),
///   unselected_prefix: String::from("  "),
/// ..Theme::default()
/// };
/// ```
#[derive(Clone, Debug)]
pub struct Theme {
    /// Prompt title color
    pub title: ColorSpec,
    /// Prompt description color
    pub description: ColorSpec,
    /// Cursor color
    pub cursor: ColorSpec,

    /// Selected option color
    pub selected_option: ColorSpec,
    /// Selected option prefix color
    pub selected_prefix: String,
    /// Selected prefix foreground color
    pub selected_prefix_fg: ColorSpec,
    /// Unselected option color
    pub unselected_option: ColorSpec,
    /// Unselected option prefix color
    pub unselected_prefix: String,
    /// Unselected prefix foreground color
    pub unselected_prefix_fg: ColorSpec,

    /// Input cursor color
    pub input_cursor: ColorSpec,
    /// Input placeholder color
    pub input_placeholder: ColorSpec,
    /// Input prompt color
    pub input_prompt: ColorSpec,

    /// Help item key color
    pub help_key: ColorSpec,
    /// Help item description color
    pub help_desc: ColorSpec,
    /// Help item separator color
    pub help_sep: ColorSpec,

    /// Focused button
    pub focused_button: ColorSpec,
    /// Blurred button
    pub blurred_button: ColorSpec,

    /// Error indicator color
    pub error_indicator: ColorSpec,
}

impl Theme {
    /// Create a new theme with no colors.
    pub fn new() -> Self {
        let placeholder = Color::Ansi256(8);

        let mut focused_button = make_color(Color::Ansi256(0));
        focused_button.set_bg(Some(Color::Ansi256(7)));

        let mut blurred_button = make_color(Color::Ansi256(7));
        blurred_button.set_bg(Some(Color::Ansi256(0)));

        Self {
            title: ColorSpec::new(),
            error_indicator: ColorSpec::new(),
            description: ColorSpec::new(),
            cursor: ColorSpec::new(),
            selected_prefix: String::from("[•]"),
            selected_prefix_fg: ColorSpec::new(),
            selected_option: ColorSpec::new(),
            unselected_prefix: String::from("[ ]"),
            unselected_prefix_fg: ColorSpec::new(),
            unselected_option: ColorSpec::new(),
            input_cursor: ColorSpec::new(),
            input_placeholder: make_color(placeholder),
            input_prompt: ColorSpec::new(),
            help_key: ColorSpec::new(),
            help_desc: ColorSpec::new(),
            help_sep: ColorSpec::new(),
            focused_button,
            blurred_button,
        }
    }

    /// Create a new theme with the charm color scheme
    pub fn charm() -> Self {
        let normal = Color::Ansi256(252);
        let indigo = Color::Rgb(117, 113, 249);
        let red = Color::Rgb(255, 70, 114);
        let fuchsia = Color::Rgb(247, 128, 226);
        let green = Color::Rgb(2, 191, 135);
        let cream = Color::Rgb(255, 253, 245);

        let mut title = make_color(indigo);
        title.set_bold(true);

        let mut focused_button = make_color(cream);
        focused_button.set_bg(Some(fuchsia));

        let mut blurred_button = make_color(normal);
        blurred_button.set_bg(Some(Color::Ansi256(238)));

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
            input_placeholder: make_color(Color::Ansi256(238)),
            input_prompt: make_color(fuchsia),

            help_key: make_color(Color::Rgb(98, 98, 98)),
            help_desc: make_color(Color::Rgb(74, 74, 74)),
            help_sep: make_color(Color::Rgb(60, 60, 60)),

            focused_button,
            blurred_button,
        }
    }

    /// Create a new theme with the dracula color scheme
    pub fn dracula() -> Self {
        let background = Color::Rgb(40, 42, 54); // #282a36
        let foreground = Color::Rgb(248, 248, 242); // #f8f8f2
        let comment = Color::Rgb(98, 114, 164); // #6272a4
        let green = Color::Rgb(80, 250, 123); // #50fa7b
        let purple = Color::Rgb(189, 147, 249); // #bd93f9
        let red = Color::Rgb(255, 85, 85); // ff5555
        let yellow = Color::Rgb(241, 250, 140); // f1fa8c

        let mut title = make_color(purple);
        title.set_bold(true);

        let mut focused_button = make_color(yellow);
        focused_button.set_bg(Some(purple));

        let mut blurred_button = make_color(foreground);
        blurred_button.set_bg(Some(background));

        Self {
            title,
            error_indicator: make_color(red),
            description: make_color(comment),
            cursor: make_color(yellow),

            selected_prefix: String::from(" [•]"),
            selected_prefix_fg: make_color(green),
            selected_option: make_color(green),
            unselected_prefix: String::from(" [ ]"),
            unselected_prefix_fg: make_color(comment),
            unselected_option: make_color(foreground),

            input_cursor: make_color(yellow),
            input_placeholder: make_color(comment),
            input_prompt: make_color(yellow),

            help_key: make_color(Color::Rgb(98, 98, 98)),
            help_desc: make_color(Color::Rgb(74, 74, 74)),
            help_sep: make_color(Color::Rgb(60, 60, 60)),

            focused_button,
            blurred_button,
        }
    }

    /// Create a new theme with the base16 color scheme
    pub fn base16() -> Self {
        let mut title = make_color(Color::Ansi256(6));
        title.set_bold(true);

        let mut focused_button = make_color(Color::Ansi256(7));
        focused_button.set_bg(Some(Color::Ansi256(5)));

        let mut blurred_button = make_color(Color::Ansi256(7));
        blurred_button.set_bg(Some(Color::Ansi256(0)));

        Self {
            title,
            error_indicator: make_color(Color::Ansi256(9)),
            description: make_color(Color::Ansi256(8)),
            cursor: make_color(Color::Ansi256(3)),

            selected_prefix: String::from(" [•]"),
            selected_prefix_fg: make_color(Color::Ansi256(2)),
            selected_option: make_color(Color::Ansi256(2)),
            unselected_prefix: String::from(" [ ]"),
            unselected_prefix_fg: make_color(Color::Ansi256(7)),
            unselected_option: make_color(Color::Ansi256(7)),

            input_cursor: make_color(Color::Ansi256(5)),
            input_placeholder: make_color(Color::Ansi256(8)),
            input_prompt: make_color(Color::Ansi256(3)),

            help_key: make_color(Color::Rgb(98, 98, 98)),
            help_desc: make_color(Color::Rgb(74, 74, 74)),
            help_sep: make_color(Color::Rgb(60, 60, 60)),

            focused_button,
            blurred_button,
        }
    }

    /// Create a new theme with the catppuccin color scheme
    pub fn catppuccin() -> Self {
        let base = Color::Rgb(30, 30, 46);
        let text = Color::Rgb(205, 214, 244);
        let subtext0 = Color::Rgb(166, 173, 200);
        let overlay0 = Color::Rgb(108, 112, 134);
        let overlay1 = Color::Rgb(127, 132, 156);
        let green = Color::Rgb(166, 227, 161);
        let red = Color::Rgb(243, 139, 168);
        let pink = Color::Rgb(245, 194, 231);
        let mauve = Color::Rgb(203, 166, 247);
        let cursor = Color::Rgb(245, 224, 220);

        let mut title = make_color(mauve);
        title.set_bold(true);

        let mut focused_button = make_color(base);
        focused_button.set_bg(Some(pink));

        let mut blurred_button = make_color(text);
        blurred_button.set_bg(Some(base));

        Self {
            title,
            error_indicator: make_color(red),
            description: make_color(subtext0),
            cursor: make_color(pink),

            selected_prefix: String::from(" [•]"),
            selected_prefix_fg: make_color(green),
            selected_option: make_color(green),
            unselected_prefix: String::from(" [ ]"),
            unselected_prefix_fg: make_color(text),
            unselected_option: make_color(text),

            input_cursor: make_color(cursor),
            input_placeholder: make_color(overlay0),
            input_prompt: make_color(pink),

            help_key: make_color(subtext0),
            help_desc: make_color(overlay1),
            help_sep: make_color(subtext0),

            focused_button,
            blurred_button,
        }
    }

    /// Create a new color with foreground color from an RGB value.
    pub fn color_rgb(r: u8, g: u8, b: u8) -> ColorSpec {
        make_color(Color::Rgb(r, g, b))
    }

    /// Create a new color with foreground color from an ANSI 256 color code.
    pub fn color_ansi256(n: u8) -> ColorSpec {
        make_color(Color::Ansi256(n))
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
