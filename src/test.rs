use std::borrow::Cow;

#[ctor::ctor]
fn init() {
    console::set_colors_enabled(false);
    console::set_colors_enabled_stderr(false);
}

pub fn without_ansi(s: &str) -> Cow<str> {
    console::strip_ansi_codes(s)
}
