use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    str::FromStr,
};

use console::{Key, Term, measure_text_width};
use demand::{Confirm, DemandOption, Input, MultiSelect, Select, Theme};
use termcolor::{Buffer, Color, ColorSpec, WriteColor};

const STORE_VERSION: &str = "1";
const STORE_FILE: &str = "theme_editor_store.txt";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MainAction {
    PreviewPalette,
    LiveDemo,
    SelectActiveTheme,
    CreateTheme,
    EditCustomTheme,
    DeleteCustomTheme,
    ExportActiveTheme,
    ImportTheme,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditAction {
    Rename,
    EditTextTokens,
    EditStyles,
    ToggleForceCursorStyle,
    PreviewPalette,
    LiveDemo,
    SaveAndReturn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextField {
    CursorStr,
    SelectedPrefix,
    UnselectedPrefix,
    BreadcrumbSeparator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StyleRole {
    Title,
    Description,
    Cursor,
    SelectedPrefix,
    SelectedOption,
    UnselectedPrefix,
    UnselectedOption,
    CursorStyle,
    InputCursor,
    InputPlaceholder,
    InputPrompt,
    HelpKey,
    HelpDesc,
    HelpSep,
    FocusedButton,
    BlurredButton,
    ErrorIndicator,
    BreadcrumbActive,
    BreadcrumbClickable,
    BreadcrumbFuture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StyleAction {
    SetForeground,
    ClearForeground,
    SetBackground,
    ClearBackground,
    ToggleBold,
    ToggleUnderline,
    Back,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StoredColor {
    Black,
    Blue,
    Green,
    Red,
    Cyan,
    Magenta,
    Yellow,
    White,
    Ansi256(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Debug)]
struct StoredStyle {
    fg: Option<StoredColor>,
    bg: Option<StoredColor>,
    bold: bool,
    underline: bool,
}

#[derive(Clone, Debug)]
struct ThemeDefinition {
    name: String,
    built_in: bool,
    title: StoredStyle,
    description: StoredStyle,
    cursor: StoredStyle,
    cursor_str: String,
    selected_prefix: String,
    selected_prefix_fg: StoredStyle,
    selected_option: StoredStyle,
    unselected_prefix: String,
    unselected_prefix_fg: StoredStyle,
    unselected_option: StoredStyle,
    cursor_style: StoredStyle,
    force_style: bool,
    input_cursor: StoredStyle,
    input_placeholder: StoredStyle,
    input_prompt: StoredStyle,
    help_key: StoredStyle,
    help_desc: StoredStyle,
    help_sep: StoredStyle,
    focused_button: StoredStyle,
    blurred_button: StoredStyle,
    error_indicator: StoredStyle,
    breadcrumb_active: StoredStyle,
    breadcrumb_clickable: StoredStyle,
    breadcrumb_future: StoredStyle,
    breadcrumb_separator: String,
}

#[derive(Clone, Debug)]
struct ThemeStore {
    active_theme: String,
    themes: Vec<ThemeDefinition>,
}

const SIMPLE_PALETTE_COLUMNS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PaletteSlot {
    TitleText,
    InputText,
    SecondaryText,
    Active,
    Inactive,
    Borders,
    Breadcrumbs,
    SelectedSuccess,
    DangerFailure,
}

#[derive(Clone, Debug)]
struct ThemePalette {
    title_text: StoredColor,
    input_text: StoredColor,
    secondary_text: StoredColor,
    active: StoredColor,
    inactive: StoredColor,
    borders: StoredColor,
    breadcrumbs: StoredColor,
    selected_success: StoredColor,
    danger_failure: StoredColor,
}

#[derive(Clone, Debug)]
struct PaletteEditorState {
    draft: ThemeDefinition,
    palette: ThemePalette,
    selected_slot: usize,
    dirty: bool,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("theme editor error: {err}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let mut store = ThemeStore::load()?;
    run_simple_editor(&mut store)
}

fn run_advanced_hub(store: &mut ThemeStore) -> io::Result<()> {
    let store_path = store_path()?;

    loop {
        let active = store.active_theme_definition().clone();
        let active_theme = active.to_theme();
        let description = format!(
            "Active: {} ({})\nCustom themes: {}\nStore: {}",
            active.name,
            if active.built_in {
                "built-in"
            } else {
                "custom"
            },
            store.custom_themes().len(),
            store_path.display()
        );

        let action = Select::new("Demand Theme Editor")
            .description(&description)
            .option(
                DemandOption::with_label(
                    "Preview current theme palette",
                    MainAction::PreviewPalette,
                )
                .description("Static breakdown of roles, tokens, and sample output"),
            )
            .option(
                DemandOption::with_label("Run live widget demo", MainAction::LiveDemo)
                    .description("Exercise real prompts with the current theme"),
            )
            .option(
                DemandOption::with_label("Select active theme", MainAction::SelectActiveTheme)
                    .description("Switch the theme currently being edited and used"),
            )
            .option(
                DemandOption::with_label("Create new custom theme", MainAction::CreateTheme)
                    .description("Clone an existing theme into a writable custom copy"),
            )
            .option(
                DemandOption::with_label("Edit a custom theme", MainAction::EditCustomTheme)
                    .description("Adjust exact role-level colors, strings, and toggles"),
            )
            .option(
                DemandOption::with_label("Delete a custom theme", MainAction::DeleteCustomTheme)
                    .description("Remove a saved custom theme from the store"),
            )
            .option(
                DemandOption::with_label("Export active theme", MainAction::ExportActiveTheme)
                    .description("Write the active theme to a reusable file"),
            )
            .option(
                DemandOption::with_label("Import theme from file", MainAction::ImportTheme)
                    .description("Load a theme file into the local store"),
            )
            .option(
                DemandOption::with_label("Exit", MainAction::Exit)
                    .description("Return to the palette-first editor"),
            )
            .theme(&active_theme)
            .run()?;

        match action {
            MainAction::PreviewPalette => preview_palette(&active)?,
            MainAction::LiveDemo => run_live_demo(&active)?,
            MainAction::SelectActiveTheme => select_active_theme(store)?,
            MainAction::CreateTheme => create_theme(store)?,
            MainAction::EditCustomTheme => edit_custom_theme(store)?,
            MainAction::DeleteCustomTheme => delete_custom_theme(store)?,
            MainAction::ExportActiveTheme => export_active_theme(store)?,
            MainAction::ImportTheme => import_theme(store)?,
            MainAction::Exit => break,
        }
    }

    Ok(())
}

fn run_simple_editor(store: &mut ThemeStore) -> io::Result<()> {
    let mut term = Term::stderr();
    term.hide_cursor()?;

    let mut state = PaletteEditorState::from_theme(store.active_theme_definition().clone());
    let result = (|| -> io::Result<()> {
        loop {
            let output = render_simple_editor(store, &state)?;
            term.clear_screen()?;
            term.write_all(output.as_bytes())?;
            term.flush()?;

            match term.read_key()? {
                Key::ArrowLeft | Key::Char('h') => state.move_left(),
                Key::ArrowRight | Key::Char('l') => state.move_right(),
                Key::ArrowUp | Key::Char('k') => state.move_up(),
                Key::ArrowDown | Key::Char('j') => state.move_down(),
                Key::Char('1') => state.selected_slot = 0,
                Key::Char('2') => state.selected_slot = 1,
                Key::Char('3') => state.selected_slot = 2,
                Key::Char('4') => state.selected_slot = 3,
                Key::Char('5') => state.selected_slot = 4,
                Key::Char('6') => state.selected_slot = 5,
                Key::Char('7') => state.selected_slot = 6,
                Key::Char('8') => state.selected_slot = 7,
                Key::Char('9') => state.selected_slot = 8,
                Key::Enter | Key::Char('e') => edit_palette_slot(store, &mut state)?,
                Key::Char('s') => save_palette_state(store, &mut state)?,
                Key::Char('t') => select_theme_from_simple(store, &mut state)?,
                Key::Char('n') => create_theme_from_simple(store, &mut state)?,
                Key::Char('d') => delete_theme_from_simple(store, &mut state)?,
                Key::Char('p') => preview_palette(&state.draft)?,
                Key::Char('v') => run_live_demo(&state.draft)?,
                Key::Char('a') => {
                    run_advanced_hub(store)?;
                    state = PaletteEditorState::from_theme(store.active_theme_definition().clone());
                }
                Key::Escape | Key::Char('q') => {
                    if confirm_discard_if_dirty(&state)? {
                        break Ok(());
                    }
                }
                _ => {}
            }
        }
    })();

    term.show_cursor()?;
    result
}

fn render_simple_editor(store: &ThemeStore, state: &PaletteEditorState) -> io::Result<String> {
    let theme = state.draft.to_theme();
    let selected = state.selected_slot();
    let mut out = Buffer::ansi();

    out.set_color(&theme.title)?;
    writeln!(out, "Demand Theme Studio")?;
    out.set_color(&theme.description)?;
    writeln!(
        out,
        "Palette mode • theme={} ({}) • custom={} • unsaved={}",
        state.draft.name,
        if state.draft.built_in {
            "built-in"
        } else {
            "custom"
        },
        store.custom_themes().len(),
        if state.dirty { "yes" } else { "no" }
    )?;
    writeln!(out)?;

    render_preview_showcase(&mut out, &theme, &state.draft.name)?;

    writeln!(out)?;
    out.set_color(&theme.help_desc)?;
    writeln!(
        out,
        "Palette: edit the nine UI roles below and the preview above updates immediately."
    )?;
    writeln!(out)?;

    render_palette_row(&mut out, state)?;
    writeln!(out)?;
    out.set_color(&theme.help_key)?;
    write!(out, "{}:", selected.label())?;
    out.set_color(&theme.help_desc)?;
    writeln!(out, " {}", selected.description())?;
    writeln!(out)?;

    out.set_color(&theme.help_key)?;
    write!(out, "enter/e")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " edit swatch ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "←/→/↑/↓")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " pick swatch ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "s")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " save ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "t")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " themes ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "n")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " new ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "d")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " delete ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "a")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " advanced ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "p/v")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " preview/demo ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "q")?;
    out.set_color(&theme.help_desc)?;
    writeln!(out, " quit")?;

    out.reset()?;
    Ok(String::from_utf8_lossy(out.as_slice()).into_owned())
}

fn edit_palette_slot(_store: &ThemeStore, state: &mut PaletteEditorState) -> io::Result<()> {
    let theme = state.draft.to_theme();
    let slot = state.selected_slot();
    let current = state.palette.color(slot);
    let color = pick_color_with_picker(
        &theme,
        &format!("Edit {} swatch", slot.label()),
        slot.description(),
        current,
    )?;
    if let Some(color) = color {
        state.palette.set(slot, color);
        state.apply_palette();
        state.dirty = true;
    }
    Ok(())
}

fn save_palette_state(store: &mut ThemeStore, state: &mut PaletteEditorState) -> io::Result<()> {
    if state.draft.built_in {
        let theme = state.draft.to_theme();
        state.draft.name = prompt_for_theme_name(&theme, store, None, "Save custom theme as")?;
        state.draft.built_in = false;
    }

    store.upsert_custom_theme(state.draft.clone());
    store.active_theme = state.draft.name.clone();
    store.save()?;
    state.dirty = false;
    Ok(())
}

fn select_theme_from_simple(
    store: &mut ThemeStore,
    state: &mut PaletteEditorState,
) -> io::Result<()> {
    if !confirm_discard_if_dirty(state)? {
        return Ok(());
    }

    let theme = state.draft.to_theme();
    let selected = Select::new("Switch theme")
        .description("Pick a theme to edit in palette mode.")
        .options(store.theme_options(Some(&state.draft.name)))
        .theme(&theme)
        .run()?;
    let next = store
        .find_theme(&selected)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "theme not found"))?
        .clone();
    store.active_theme = next.name.clone();
    *state = PaletteEditorState::from_theme(next);
    Ok(())
}

fn create_theme_from_simple(
    store: &mut ThemeStore,
    state: &mut PaletteEditorState,
) -> io::Result<()> {
    let theme = state.draft.to_theme();
    let mut draft = state.draft.clone();
    draft.name = prompt_for_theme_name(&theme, store, None, "New theme name")?;
    draft.built_in = false;
    *state = PaletteEditorState::from_theme(draft);
    state.dirty = true;
    Ok(())
}

fn delete_theme_from_simple(
    store: &mut ThemeStore,
    state: &mut PaletteEditorState,
) -> io::Result<()> {
    if state.draft.built_in {
        let theme = state.draft.to_theme();
        return pause(
            &theme,
            "Built-in theme",
            "Built-in themes cannot be deleted. Save a custom copy first.",
        );
    }

    let theme = state.draft.to_theme();
    let confirm = Confirm::new("Delete current custom theme?")
        .description(&format!("This will remove '{}'.", state.draft.name))
        .affirmative("Delete")
        .negative("Keep")
        .theme(&theme)
        .run()?;

    if confirm {
        let deleted_name = state.draft.name.clone();
        store.remove_custom_theme(&deleted_name);
        let fallback = store
            .find_theme("charm")
            .or_else(|| store.themes.first())
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no fallback theme"))?;
        store.active_theme = fallback.name.clone();
        store.save()?;
        *state = PaletteEditorState::from_theme(fallback);
    }

    Ok(())
}

fn confirm_discard_if_dirty(state: &PaletteEditorState) -> io::Result<bool> {
    if !state.dirty {
        return Ok(true);
    }

    let theme = state.draft.to_theme();
    Confirm::new("Discard unsaved palette changes?")
        .description("Your live preview changes have not been saved yet.")
        .affirmative("Discard")
        .negative("Keep editing")
        .theme(&theme)
        .run()
}

fn render_preview_showcase(out: &mut Buffer, theme: &Theme, theme_name: &str) -> io::Result<()> {
    const WIDTH: usize = 62;
    let breadcrumb_sep = format!(" {} ", theme.breadcrumb_separator.trim());
    let breadcrumb_full_width = measure_text_width("Themes")
        + measure_text_width(&breadcrumb_sep)
        + measure_text_width("Preview")
        + measure_text_width(&breadcrumb_sep)
        + measure_text_width("Save");
    let breadcrumb_partial_width = measure_text_width("Themes")
        + measure_text_width(&breadcrumb_sep)
        + measure_text_width("Preview");

    write_box_top(out, &theme.help_sep, WIDTH, "")?;
    write_box_line_single(
        out,
        &theme.help_sep,
        &theme.title,
        "Demand Theme Studio Preview",
        WIDTH,
    )?;
    write_box_line_single(
        out,
        &theme.help_sep,
        &theme.description,
        &format!("Editing theme: {theme_name}"),
        WIDTH,
    )?;
    write_box_separator(out, &theme.help_sep, WIDTH)?;

    out.set_color(&theme.help_sep)?;
    write!(out, "│ ")?;
    out.set_color(&theme.input_prompt)?;
    write!(out, "Email: ")?;
    out.set_color(&theme.unselected_option)?;
    write!(out, "name@example.com ")?;
    out.set_color(&theme.real_cursor_color(Some(&theme.input_placeholder)))?;
    write!(out, " ")?;
    out.reset()?;
    write_box_fill(out, WIDTH, "Email: ".len() + "name@example.com ".len() + 1)?;
    out.set_color(&theme.help_sep)?;
    writeln!(out, " │")?;

    out.set_color(&theme.help_sep)?;
    write!(out, "│ ")?;
    out.set_color(&theme.cursor)?;
    write!(out, "{} ", theme.cursor_str)?;
    out.set_color(&theme.selected_prefix_fg)?;
    write!(out, "{} ", theme.selected_prefix)?;
    out.set_color(&theme.selected_option)?;
    write!(out, "Selected option")?;
    out.reset()?;
    write_box_fill(
        out,
        WIDTH,
        measure_text_width(&theme.cursor_str)
            + 1
            + measure_text_width(&theme.selected_prefix)
            + 1
            + "Selected option".len(),
    )?;
    out.set_color(&theme.help_sep)?;
    writeln!(out, " │")?;

    out.set_color(&theme.help_sep)?;
    write!(out, "│ ")?;
    out.set_color(&theme.unselected_prefix_fg)?;
    write!(out, "{} ", theme.unselected_prefix)?;
    out.set_color(&theme.unselected_option)?;
    write!(out, "Another option")?;
    out.reset()?;
    write_box_fill(
        out,
        WIDTH,
        measure_text_width(&theme.unselected_prefix) + 1 + "Another option".len(),
    )?;
    out.set_color(&theme.help_sep)?;
    writeln!(out, " │")?;

    write_box_separator(out, &theme.help_sep, WIDTH)?;

    out.set_color(&theme.help_sep)?;
    write!(out, "│ ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "enter")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " confirm ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "esc")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " cancel")?;
    out.reset()?;
    write_box_fill(out, WIDTH, "enter confirm • esc cancel".len())?;
    out.set_color(&theme.help_sep)?;
    writeln!(out, " │")?;

    out.set_color(&theme.help_sep)?;
    write!(out, "│ ")?;
    out.set_color(&theme.focused_button)?;
    write!(out, "  Save  ")?;
    out.reset()?;
    write!(out, " ")?;
    out.set_color(&theme.blurred_button)?;
    write!(out, "  Cancel  ")?;
    out.reset()?;
    write_box_fill(out, WIDTH, "  Save     Cancel  ".len())?;
    out.set_color(&theme.help_sep)?;
    writeln!(out, " │")?;

    out.set_color(&theme.help_sep)?;
    write!(out, "│ ")?;
    out.set_color(&theme.selected_option)?;
    write!(out, "✓ Saved")?;
    out.reset()?;
    write!(out, "  ")?;
    out.set_color(&theme.error_indicator)?;
    write!(out, "✗ Validation failed")?;
    out.reset()?;
    write_box_fill(
        out,
        WIDTH,
        measure_text_width("✓ Saved") + 2 + measure_text_width("✗ Validation failed"),
    )?;
    out.set_color(&theme.help_sep)?;
    writeln!(out, " │")?;

    write_box_separator(out, &theme.help_sep, WIDTH)?;

    out.set_color(&theme.help_sep)?;
    write!(out, "│ ")?;
    out.set_color(&theme.breadcrumb_clickable)?;
    write!(out, "Themes")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "{breadcrumb_sep}")?;
    out.set_color(&theme.breadcrumb_active)?;
    write!(out, "Preview")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "{breadcrumb_sep}")?;
    out.set_color(&theme.breadcrumb_future)?;
    write!(out, "Save")?;
    out.reset()?;
    write_box_fill(out, WIDTH, breadcrumb_full_width)?;
    out.set_color(&theme.help_sep)?;
    writeln!(out, " │")?;

    out.set_color(&theme.help_sep)?;
    write!(out, "│ ")?;
    out.set_color(&theme.breadcrumb_clickable)?;
    write!(out, "Themes")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "{breadcrumb_sep}")?;
    out.set_color(&theme.breadcrumb_active)?;
    write!(out, "Preview")?;
    out.reset()?;
    write_box_fill(out, WIDTH, breadcrumb_partial_width)?;
    out.set_color(&theme.help_sep)?;
    writeln!(out, " │")?;

    write_box_bottom(out, &theme.help_sep, WIDTH)?;
    Ok(())
}

fn write_box_top(
    out: &mut Buffer,
    border: &ColorSpec,
    width: usize,
    label: &str,
) -> io::Result<()> {
    out.set_color(border)?;
    let line = format!("┌{}┐", "─".repeat(width + 2));
    writeln!(out, "{line}")?;
    out.set_color(border)?;
    write!(out, "│ ")?;
    out.reset()?;
    write!(out, "{label}")?;
    write_box_fill(out, width, measure_text_width(label))?;
    out.set_color(border)?;
    writeln!(out, " │")?;
    Ok(())
}

fn write_box_bottom(out: &mut Buffer, border: &ColorSpec, width: usize) -> io::Result<()> {
    out.set_color(border)?;
    writeln!(out, "└{}┘", "─".repeat(width + 2))?;
    Ok(())
}

fn write_box_separator(out: &mut Buffer, border: &ColorSpec, width: usize) -> io::Result<()> {
    out.set_color(border)?;
    writeln!(out, "├{}┤", "─".repeat(width + 2))?;
    Ok(())
}

fn write_box_line_single(
    out: &mut Buffer,
    border: &ColorSpec,
    style: &ColorSpec,
    text: &str,
    width: usize,
) -> io::Result<()> {
    out.set_color(border)?;
    write!(out, "│ ")?;
    out.set_color(style)?;
    write!(out, "{text}")?;
    out.reset()?;
    write_box_fill(out, width, measure_text_width(text))?;
    out.set_color(border)?;
    writeln!(out, " │")?;
    Ok(())
}

fn write_box_fill(out: &mut Buffer, width: usize, content_width: usize) -> io::Result<()> {
    let remaining = width.saturating_sub(content_width);
    write!(out, "{}", " ".repeat(remaining))
}

fn render_palette_row(out: &mut Buffer, state: &PaletteEditorState) -> io::Result<()> {
    let slots = PaletteSlot::all();
    let rows = slots.len().div_ceil(SIMPLE_PALETTE_COLUMNS);
    for row in 0..rows {
        for col in 0..SIMPLE_PALETTE_COLUMNS {
            let index = row * SIMPLE_PALETTE_COLUMNS + col;
            if index >= slots.len() {
                break;
            }
            let slot = slots[index];
            let color = state.palette.color(slot);
            let style = swatch_style(color);
            if index == state.selected_slot {
                out.reset()?;
                write!(out, "> ")?;
            } else {
                out.reset()?;
                write!(out, "  ")?;
            }
            out.set_color(&style)?;
            write!(out, " {} {} ", index + 1, slot.short_label())?;
            out.reset()?;
            write!(out, " {}", color.display_value())?;
            if col + 1 < SIMPLE_PALETTE_COLUMNS && index + 1 < slots.len() {
                write!(out, "   ")?;
            }
        }
        writeln!(out)?;
    }
    Ok(())
}

impl ThemeStore {
    fn load() -> io::Result<Self> {
        let mut store = Self::builtin();
        let path = store_path()?;
        if !path.exists() {
            return Ok(store);
        }

        let contents = fs::read_to_string(path)?;
        let mut current_theme: Option<ThemeDefinition> = None;

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if line == "[theme]" {
                if let Some(theme) = current_theme.take() {
                    store.upsert_custom_theme(theme);
                }
                current_theme = Some(ThemeDefinition::blank_custom());
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let value = unescape(value);
            match current_theme.as_mut() {
                Some(theme) => theme.apply_persisted_field(key, &value)?,
                None => match key {
                    "version" => {
                        if value != STORE_VERSION {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("unsupported theme store version: {value}"),
                            ));
                        }
                    }
                    "active_theme" => store.active_theme = value,
                    _ => {}
                },
            }
        }

        if let Some(theme) = current_theme.take() {
            store.upsert_custom_theme(theme);
        }

        if store.find_theme(&store.active_theme).is_none() {
            store.active_theme = String::from("charm");
        }

        Ok(store)
    }

    fn builtin() -> Self {
        Self {
            active_theme: String::from("charm"),
            themes: vec![
                ThemeDefinition::from_theme("new", true, &Theme::new()),
                ThemeDefinition::from_theme("charm", true, &Theme::charm()),
                ThemeDefinition::from_theme("dracula", true, &Theme::dracula()),
                ThemeDefinition::from_theme("catppuccin", true, &Theme::catppuccin()),
                ThemeDefinition::from_theme("base16", true, &Theme::base16()),
            ],
        }
    }

    fn save(&self) -> io::Result<()> {
        let path = store_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out = String::new();
        out.push_str(&format!("version={STORE_VERSION}\n"));
        out.push_str(&format!("active_theme={}\n", escape(&self.active_theme)));

        for theme in self.themes.iter().filter(|theme| !theme.built_in) {
            out.push_str("\n[theme]\n");
            out.push_str(&theme.to_persisted_block());
        }

        fs::write(path, out)
    }

    fn upsert_custom_theme(&mut self, theme: ThemeDefinition) {
        if let Some(existing) = self
            .themes
            .iter_mut()
            .find(|candidate| !candidate.built_in && candidate.name == theme.name)
        {
            *existing = theme;
        } else {
            self.themes.push(theme);
        }
    }

    fn active_theme_definition(&self) -> &ThemeDefinition {
        self.find_theme(&self.active_theme)
            .unwrap_or(&self.themes[0])
    }

    fn find_theme(&self, name: &str) -> Option<&ThemeDefinition> {
        self.themes.iter().find(|theme| theme.name == name)
    }

    fn custom_themes(&self) -> Vec<&ThemeDefinition> {
        self.themes.iter().filter(|theme| !theme.built_in).collect()
    }

    fn remove_custom_theme(&mut self, name: &str) {
        self.themes
            .retain(|theme| theme.built_in || theme.name != name);
    }
}

impl ThemeDefinition {
    fn blank_custom() -> Self {
        Self::from_theme("untitled", false, &Theme::new())
    }

    fn from_theme(name: &str, built_in: bool, theme: &Theme) -> Self {
        Self {
            name: name.to_string(),
            built_in,
            title: StoredStyle::from_spec(&theme.title),
            description: StoredStyle::from_spec(&theme.description),
            cursor: StoredStyle::from_spec(&theme.cursor),
            cursor_str: theme.cursor_str.clone(),
            selected_prefix: theme.selected_prefix.clone(),
            selected_prefix_fg: StoredStyle::from_spec(&theme.selected_prefix_fg),
            selected_option: StoredStyle::from_spec(&theme.selected_option),
            unselected_prefix: theme.unselected_prefix.clone(),
            unselected_prefix_fg: StoredStyle::from_spec(&theme.unselected_prefix_fg),
            unselected_option: StoredStyle::from_spec(&theme.unselected_option),
            cursor_style: StoredStyle::from_spec(&theme.cursor_style),
            force_style: theme.force_style,
            input_cursor: StoredStyle::from_spec(&theme.input_cursor),
            input_placeholder: StoredStyle::from_spec(&theme.input_placeholder),
            input_prompt: StoredStyle::from_spec(&theme.input_prompt),
            help_key: StoredStyle::from_spec(&theme.help_key),
            help_desc: StoredStyle::from_spec(&theme.help_desc),
            help_sep: StoredStyle::from_spec(&theme.help_sep),
            focused_button: StoredStyle::from_spec(&theme.focused_button),
            blurred_button: StoredStyle::from_spec(&theme.blurred_button),
            error_indicator: StoredStyle::from_spec(&theme.error_indicator),
            breadcrumb_active: StoredStyle::from_spec(&theme.breadcrumb_active),
            breadcrumb_clickable: StoredStyle::from_spec(&theme.breadcrumb_clickable),
            breadcrumb_future: StoredStyle::from_spec(&theme.breadcrumb_future),
            breadcrumb_separator: theme.breadcrumb_separator.clone(),
        }
    }

    fn to_theme(&self) -> Theme {
        let mut theme = Theme::new();
        theme.title = self.title.to_spec();
        theme.description = self.description.to_spec();
        theme.cursor = self.cursor.to_spec();
        theme.cursor_str = self.cursor_str.clone();
        theme.selected_option = self.selected_option.to_spec();
        theme.selected_prefix = self.selected_prefix.clone();
        theme.selected_prefix_fg = self.selected_prefix_fg.to_spec();
        theme.unselected_option = self.unselected_option.to_spec();
        theme.unselected_prefix = self.unselected_prefix.clone();
        theme.unselected_prefix_fg = self.unselected_prefix_fg.to_spec();
        theme.cursor_style = self.cursor_style.to_spec();
        theme.force_style = self.force_style;
        theme.input_cursor = self.input_cursor.to_spec();
        theme.input_placeholder = self.input_placeholder.to_spec();
        theme.input_prompt = self.input_prompt.to_spec();
        theme.help_key = self.help_key.to_spec();
        theme.help_desc = self.help_desc.to_spec();
        theme.help_sep = self.help_sep.to_spec();
        theme.focused_button = self.focused_button.to_spec();
        theme.blurred_button = self.blurred_button.to_spec();
        theme.error_indicator = self.error_indicator.to_spec();
        theme.breadcrumb_active = self.breadcrumb_active.to_spec();
        theme.breadcrumb_clickable = self.breadcrumb_clickable.to_spec();
        theme.breadcrumb_future = self.breadcrumb_future.to_spec();
        theme.breadcrumb_separator = self.breadcrumb_separator.clone();
        theme
    }

    fn to_persisted_block(&self) -> String {
        let mut out = String::new();

        self.push_field(&mut out, "name", &self.name);
        self.push_field(
            &mut out,
            "force_style",
            if self.force_style { "true" } else { "false" },
        );
        self.push_field(&mut out, "cursor_str", &self.cursor_str);
        self.push_field(&mut out, "selected_prefix", &self.selected_prefix);
        self.push_field(&mut out, "unselected_prefix", &self.unselected_prefix);
        self.push_field(&mut out, "breadcrumb_separator", &self.breadcrumb_separator);

        self.push_style_field(&mut out, "title", &self.title);
        self.push_style_field(&mut out, "description", &self.description);
        self.push_style_field(&mut out, "cursor", &self.cursor);
        self.push_style_field(&mut out, "selected_prefix_fg", &self.selected_prefix_fg);
        self.push_style_field(&mut out, "selected_option", &self.selected_option);
        self.push_style_field(&mut out, "unselected_prefix_fg", &self.unselected_prefix_fg);
        self.push_style_field(&mut out, "unselected_option", &self.unselected_option);
        self.push_style_field(&mut out, "cursor_style", &self.cursor_style);
        self.push_style_field(&mut out, "input_cursor", &self.input_cursor);
        self.push_style_field(&mut out, "input_placeholder", &self.input_placeholder);
        self.push_style_field(&mut out, "input_prompt", &self.input_prompt);
        self.push_style_field(&mut out, "help_key", &self.help_key);
        self.push_style_field(&mut out, "help_desc", &self.help_desc);
        self.push_style_field(&mut out, "help_sep", &self.help_sep);
        self.push_style_field(&mut out, "focused_button", &self.focused_button);
        self.push_style_field(&mut out, "blurred_button", &self.blurred_button);
        self.push_style_field(&mut out, "error_indicator", &self.error_indicator);
        self.push_style_field(&mut out, "breadcrumb_active", &self.breadcrumb_active);
        self.push_style_field(&mut out, "breadcrumb_clickable", &self.breadcrumb_clickable);
        self.push_style_field(&mut out, "breadcrumb_future", &self.breadcrumb_future);

        out
    }

    fn push_field(&self, out: &mut String, key: &str, value: &str) {
        out.push_str(key);
        out.push('=');
        out.push_str(&escape(value));
        out.push('\n');
    }

    fn push_style_field(&self, out: &mut String, key: &str, style: &StoredStyle) {
        self.push_field(out, key, &style.serialize());
    }

    fn apply_persisted_field(&mut self, key: &str, value: &str) -> io::Result<()> {
        match key {
            "name" => self.name = value.to_string(),
            "cursor_shape" => {}
            "force_style" => self.force_style = parse_bool(value)?,
            "cursor_str" => self.cursor_str = value.to_string(),
            "selected_prefix" => self.selected_prefix = value.to_string(),
            "unselected_prefix" => self.unselected_prefix = value.to_string(),
            "breadcrumb_separator" => self.breadcrumb_separator = value.to_string(),
            "title" => self.title = StoredStyle::deserialize(value)?,
            "description" => self.description = StoredStyle::deserialize(value)?,
            "cursor" => self.cursor = StoredStyle::deserialize(value)?,
            "selected_prefix_fg" => self.selected_prefix_fg = StoredStyle::deserialize(value)?,
            "selected_option" => self.selected_option = StoredStyle::deserialize(value)?,
            "unselected_prefix_fg" => self.unselected_prefix_fg = StoredStyle::deserialize(value)?,
            "unselected_option" => self.unselected_option = StoredStyle::deserialize(value)?,
            "cursor_style" => self.cursor_style = StoredStyle::deserialize(value)?,
            "input_cursor" => self.input_cursor = StoredStyle::deserialize(value)?,
            "input_placeholder" => self.input_placeholder = StoredStyle::deserialize(value)?,
            "input_prompt" => self.input_prompt = StoredStyle::deserialize(value)?,
            "help_key" => self.help_key = StoredStyle::deserialize(value)?,
            "help_desc" => self.help_desc = StoredStyle::deserialize(value)?,
            "help_sep" => self.help_sep = StoredStyle::deserialize(value)?,
            "focused_button" => self.focused_button = StoredStyle::deserialize(value)?,
            "blurred_button" => self.blurred_button = StoredStyle::deserialize(value)?,
            "error_indicator" => self.error_indicator = StoredStyle::deserialize(value)?,
            "breadcrumb_active" => self.breadcrumb_active = StoredStyle::deserialize(value)?,
            "breadcrumb_clickable" => self.breadcrumb_clickable = StoredStyle::deserialize(value)?,
            "breadcrumb_future" => self.breadcrumb_future = StoredStyle::deserialize(value)?,
            _ => {}
        }

        Ok(())
    }

    fn to_export_document(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("version={STORE_VERSION}\n"));
        out.push_str("[theme]\n");
        out.push_str(&self.to_persisted_block());
        out
    }

    fn from_export_document(contents: &str) -> io::Result<Self> {
        let mut theme: Option<ThemeDefinition> = None;

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if line == "[theme]" {
                if theme.is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "theme export should contain exactly one [theme] section",
                    ));
                }
                theme = Some(ThemeDefinition::blank_custom());
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = unescape(value);

            match theme.as_mut() {
                Some(theme) => theme.apply_persisted_field(key, &value)?,
                None => {
                    if key == "version" && value != STORE_VERSION {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("unsupported theme export version: {value}"),
                        ));
                    }
                }
            }
        }

        let mut theme = theme.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "theme export is missing a [theme] section",
            )
        })?;
        theme.built_in = false;
        Ok(theme)
    }
}

impl StoredStyle {
    fn from_spec(spec: &ColorSpec) -> Self {
        Self {
            fg: spec.fg().map(StoredColor::from_termcolor),
            bg: spec.bg().map(StoredColor::from_termcolor),
            bold: spec.bold(),
            underline: spec.underline(),
        }
    }

    fn to_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(self.fg.as_ref().map(StoredColor::to_termcolor));
        spec.set_bg(self.bg.as_ref().map(StoredColor::to_termcolor));
        spec.set_bold(self.bold);
        spec.set_underline(self.underline);
        spec
    }

    fn describe(&self) -> String {
        let fg = self
            .fg
            .as_ref()
            .map(StoredColor::serialize)
            .unwrap_or_else(|| String::from("none"));
        let bg = self
            .bg
            .as_ref()
            .map(StoredColor::serialize)
            .unwrap_or_else(|| String::from("none"));
        format!(
            "fg={fg}, bg={bg}, bold={}, underline={}",
            self.bold, self.underline
        )
    }

    fn serialize(&self) -> String {
        let fg = self
            .fg
            .as_ref()
            .map(StoredColor::serialize)
            .unwrap_or_else(|| String::from("none"));
        let bg = self
            .bg
            .as_ref()
            .map(StoredColor::serialize)
            .unwrap_or_else(|| String::from("none"));
        format!(
            "fg:{fg};bg:{bg};bold:{};underline:{}",
            bool_flag(self.bold),
            bool_flag(self.underline)
        )
    }

    fn deserialize(input: &str) -> io::Result<Self> {
        let mut style = Self {
            fg: None,
            bg: None,
            bold: false,
            underline: false,
        };

        for part in input.split(';') {
            let Some((key, value)) = part.split_once(':') else {
                continue;
            };
            match key {
                "fg" => style.fg = StoredColor::deserialize_optional(value)?,
                "bg" => style.bg = StoredColor::deserialize_optional(value)?,
                "bold" => style.bold = value == "1",
                "underline" => style.underline = value == "1",
                _ => {}
            }
        }

        Ok(style)
    }
}

impl StoredColor {
    fn from_termcolor(color: &Color) -> Self {
        match color {
            Color::Black => Self::Black,
            Color::Blue => Self::Blue,
            Color::Green => Self::Green,
            Color::Red => Self::Red,
            Color::Cyan => Self::Cyan,
            Color::Magenta => Self::Magenta,
            Color::Yellow => Self::Yellow,
            Color::White => Self::White,
            Color::Ansi256(value) => Self::Ansi256(*value),
            Color::Rgb(r, g, b) => Self::Rgb(*r, *g, *b),
            // `termcolor::Color` is non-exhaustive; any future variant not
            // handled here is conservatively mapped to `White`.
            _ => Self::White,
        }
    }

    fn to_termcolor(&self) -> Color {
        match self {
            Self::Black => Color::Black,
            Self::Blue => Color::Blue,
            Self::Green => Color::Green,
            Self::Red => Color::Red,
            Self::Cyan => Color::Cyan,
            Self::Magenta => Color::Magenta,
            Self::Yellow => Color::Yellow,
            Self::White => Color::White,
            Self::Ansi256(value) => Color::Ansi256(*value),
            Self::Rgb(r, g, b) => Color::Rgb(*r, *g, *b),
        }
    }

    fn serialize(&self) -> String {
        match self {
            Self::Black => String::from("black"),
            Self::Blue => String::from("blue"),
            Self::Green => String::from("green"),
            Self::Red => String::from("red"),
            Self::Cyan => String::from("cyan"),
            Self::Magenta => String::from("magenta"),
            Self::Yellow => String::from("yellow"),
            Self::White => String::from("white"),
            Self::Ansi256(value) => format!("ansi:{value}"),
            Self::Rgb(r, g, b) => format!("rgb:{r}:{g}:{b}"),
        }
    }

    fn deserialize_optional(input: &str) -> io::Result<Option<Self>> {
        if input == "none" {
            return Ok(None);
        }

        Ok(Some(Self::deserialize(input)?))
    }

    fn deserialize(input: &str) -> io::Result<Self> {
        if let Some(value) = input.strip_prefix("ansi:") {
            let parsed = value.parse::<u8>().map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid ansi256 color '{input}': {err}"),
                )
            })?;
            return Ok(Self::Ansi256(parsed));
        }

        if let Some(value) = input.strip_prefix("rgb:") {
            let parts = value.split(':').collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid rgb color '{input}'"),
                ));
            }
            let r = parts[0].parse::<u8>().map_err(invalid_rgb(input))?;
            let g = parts[1].parse::<u8>().map_err(invalid_rgb(input))?;
            let b = parts[2].parse::<u8>().map_err(invalid_rgb(input))?;
            return Ok(Self::Rgb(r, g, b));
        }

        match input {
            "black" => Ok(Self::Black),
            "blue" => Ok(Self::Blue),
            "green" => Ok(Self::Green),
            "red" => Ok(Self::Red),
            "cyan" => Ok(Self::Cyan),
            "magenta" => Ok(Self::Magenta),
            "yellow" => Ok(Self::Yellow),
            "white" => Ok(Self::White),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid color token '{input}'"),
            )),
        }
    }

    fn display_value(&self) -> String {
        match self {
            Self::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => self.serialize(),
        }
    }

    fn rgb_triplet(&self) -> (u8, u8, u8) {
        match self {
            Self::Black => (0, 0, 0),
            Self::Blue => (0, 0, 255),
            Self::Green => (0, 128, 0),
            Self::Red => (255, 0, 0),
            Self::Cyan => (0, 255, 255),
            Self::Magenta => (255, 0, 255),
            Self::Yellow => (255, 255, 0),
            Self::White => (255, 255, 255),
            Self::Ansi256(value) => ansi256_rgb(*value),
            Self::Rgb(r, g, b) => (*r, *g, *b),
        }
    }

    fn ansi_seed(&self) -> u8 {
        match self {
            Self::Ansi256(value) => *value,
            _ => {
                let (r, g, b) = self.rgb_triplet();
                nearest_ansi256(r, g, b)
            }
        }
    }
}

impl PaletteSlot {
    fn all() -> [PaletteSlot; 9] {
        [
            PaletteSlot::TitleText,
            PaletteSlot::InputText,
            PaletteSlot::SecondaryText,
            PaletteSlot::Active,
            PaletteSlot::Inactive,
            PaletteSlot::Borders,
            PaletteSlot::Breadcrumbs,
            PaletteSlot::SelectedSuccess,
            PaletteSlot::DangerFailure,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            PaletteSlot::TitleText => "Title Text",
            PaletteSlot::InputText => "Input Text",
            PaletteSlot::SecondaryText => "Secondary Text",
            PaletteSlot::Active => "Active",
            PaletteSlot::Inactive => "Inactive",
            PaletteSlot::Borders => "Borders/Separators",
            PaletteSlot::Breadcrumbs => "Breadcrumbs",
            PaletteSlot::SelectedSuccess => "Selected/Success",
            PaletteSlot::DangerFailure => "Danger/Failure",
        }
    }

    fn short_label(self) -> &'static str {
        match self {
            PaletteSlot::TitleText => "Title",
            PaletteSlot::InputText => "Input",
            PaletteSlot::SecondaryText => "Secondary",
            PaletteSlot::Active => "Active",
            PaletteSlot::Inactive => "Inactive",
            PaletteSlot::Borders => "Borders",
            PaletteSlot::Breadcrumbs => "Crumbs",
            PaletteSlot::SelectedSuccess => "Selected",
            PaletteSlot::DangerFailure => "Danger",
        }
    }

    fn description(self) -> &'static str {
        match self {
            PaletteSlot::TitleText => "Used for titles and other headline-style text.",
            PaletteSlot::InputText => {
                "Used for input labels, entered values, and main interactive text."
            }
            PaletteSlot::SecondaryText => {
                "Used for subtext, placeholders, and lower-emphasis explanatory copy."
            }
            PaletteSlot::Active => "Used for cursors and focused interactive emphasis.",
            PaletteSlot::Inactive => "Used for unfocused surfaces and subdued filled UI elements.",
            PaletteSlot::Borders => "Used for borders, dividers, and structural separators.",
            PaletteSlot::Breadcrumbs => "Used for breadcrumb text.",
            PaletteSlot::SelectedSuccess => {
                "Used for selected states and positive confirmation cues."
            }
            PaletteSlot::DangerFailure => "Used for errors and destructive failure states.",
        }
    }
}

impl ThemePalette {
    fn from_definition(theme: &ThemeDefinition) -> Self {
        Self {
            title_text: theme.title.fg.unwrap_or(StoredColor::White),
            input_text: theme
                .input_prompt
                .fg
                .or(theme.unselected_option.fg)
                .unwrap_or(StoredColor::White),
            secondary_text: theme
                .description
                .fg
                .or(theme.input_placeholder.fg)
                .unwrap_or(StoredColor::Ansi256(8)),
            active: theme
                .cursor
                .fg
                .or(theme.input_cursor.fg)
                .unwrap_or(StoredColor::White),
            inactive: theme
                .blurred_button
                .bg
                .or(theme.cursor_style.bg)
                .unwrap_or(StoredColor::Black),
            borders: theme.help_sep.fg.unwrap_or(StoredColor::Ansi256(8)),
            breadcrumbs: theme
                .breadcrumb_clickable
                .fg
                .or(theme.breadcrumb_active.fg)
                .unwrap_or(StoredColor::Ansi256(8)),
            selected_success: theme
                .selected_option
                .fg
                .or(theme.selected_prefix_fg.fg)
                .unwrap_or(StoredColor::Green),
            danger_failure: theme.error_indicator.fg.unwrap_or(StoredColor::Red),
        }
    }

    fn color(&self, slot: PaletteSlot) -> StoredColor {
        match slot {
            PaletteSlot::TitleText => self.title_text,
            PaletteSlot::InputText => self.input_text,
            PaletteSlot::SecondaryText => self.secondary_text,
            PaletteSlot::Active => self.active,
            PaletteSlot::Inactive => self.inactive,
            PaletteSlot::Borders => self.borders,
            PaletteSlot::Breadcrumbs => self.breadcrumbs,
            PaletteSlot::SelectedSuccess => self.selected_success,
            PaletteSlot::DangerFailure => self.danger_failure,
        }
    }

    fn set(&mut self, slot: PaletteSlot, color: StoredColor) {
        match slot {
            PaletteSlot::TitleText => self.title_text = color,
            PaletteSlot::InputText => self.input_text = color,
            PaletteSlot::SecondaryText => self.secondary_text = color,
            PaletteSlot::Active => self.active = color,
            PaletteSlot::Inactive => self.inactive = color,
            PaletteSlot::Borders => self.borders = color,
            PaletteSlot::Breadcrumbs => self.breadcrumbs = color,
            PaletteSlot::SelectedSuccess => self.selected_success = color,
            PaletteSlot::DangerFailure => self.danger_failure = color,
        }
    }
}

impl PaletteEditorState {
    fn from_theme(theme: ThemeDefinition) -> Self {
        let palette = ThemePalette::from_definition(&theme);
        Self {
            draft: theme,
            palette,
            selected_slot: 0,
            dirty: false,
        }
    }

    fn selected_slot(&self) -> PaletteSlot {
        PaletteSlot::all()[self.selected_slot]
    }

    fn move_left(&mut self) {
        if self.selected_slot == 0 {
            self.selected_slot = PaletteSlot::all().len() - 1;
        } else {
            self.selected_slot -= 1;
        }
    }

    fn move_right(&mut self) {
        self.selected_slot = (self.selected_slot + 1) % PaletteSlot::all().len();
    }

    fn move_up(&mut self) {
        if self.selected_slot >= SIMPLE_PALETTE_COLUMNS {
            self.selected_slot -= SIMPLE_PALETTE_COLUMNS;
        }
    }

    fn move_down(&mut self) {
        if self.selected_slot + SIMPLE_PALETTE_COLUMNS < PaletteSlot::all().len() {
            self.selected_slot += SIMPLE_PALETTE_COLUMNS;
        }
    }

    fn apply_palette(&mut self) {
        let active_contrast = contrasting_color(self.palette.active);
        let inactive_contrast = contrasting_color(self.palette.inactive);

        self.draft.title = style_fg(self.palette.title_text, true);
        self.draft.description = style_fg(self.palette.secondary_text, false);
        self.draft.cursor = style_fg(self.palette.active, false);
        self.draft.selected_prefix_fg = style_fg(self.palette.selected_success, false);
        self.draft.selected_option = style_fg(self.palette.selected_success, false);
        self.draft.unselected_prefix_fg = style_fg(self.palette.secondary_text, false);
        self.draft.unselected_option = style_fg(self.palette.input_text, false);
        self.draft.cursor_style = StoredStyle {
            fg: Some(self.palette.input_text),
            bg: Some(self.palette.inactive),
            bold: false,
            underline: false,
        };
        self.draft.input_cursor = style_fg(self.palette.active, false);
        self.draft.input_placeholder = style_fg(self.palette.secondary_text, false);
        self.draft.input_prompt = style_fg(self.palette.input_text, false);
        self.draft.help_key = style_fg(self.palette.input_text, false);
        self.draft.help_desc = style_fg(self.palette.secondary_text, false);
        self.draft.help_sep = style_fg(self.palette.borders, false);
        self.draft.focused_button = StoredStyle {
            fg: Some(active_contrast),
            bg: Some(self.palette.active),
            bold: true,
            underline: false,
        };
        self.draft.blurred_button = StoredStyle {
            fg: Some(inactive_contrast),
            bg: Some(self.palette.inactive),
            bold: false,
            underline: false,
        };
        self.draft.error_indicator = style_fg(self.palette.danger_failure, false);
        self.draft.breadcrumb_active = style_fg(self.palette.active, false);
        self.draft.breadcrumb_clickable = style_fg(self.palette.breadcrumbs, false);
        self.draft.breadcrumb_future = style_fg(self.palette.secondary_text, false);
    }
}

fn style_fg(color: StoredColor, bold: bool) -> StoredStyle {
    StoredStyle {
        fg: Some(color),
        bg: None,
        bold,
        underline: false,
    }
}

fn swatch_style(color: StoredColor) -> ColorSpec {
    let mut style = ColorSpec::new();
    style.set_bg(Some(color.to_termcolor()));
    style.set_fg(Some(contrasting_color(color).to_termcolor()));
    style.set_bold(true);
    style
}

fn shade_cell_style(color: StoredColor) -> ColorSpec {
    let mut style = ColorSpec::new();
    style.set_fg(Some(color.to_termcolor()));
    style
}

fn contrasting_color(color: StoredColor) -> StoredColor {
    let (r, g, b) = color.rgb_triplet();
    let brightness = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000;
    if brightness > 140 {
        StoredColor::Black
    } else {
        StoredColor::White
    }
}

fn preview_palette(theme_def: &ThemeDefinition) -> io::Result<()> {
    let output = render_theme_preview(theme_def)?;
    let mut term = Term::stderr();
    term.clear_screen()?;
    term.write_all(output.as_bytes())?;
    term.flush()?;

    let theme = theme_def.to_theme();
    pause(
        &theme,
        "Palette preview complete",
        "Press enter to return to the editor.",
    )
}

fn run_live_demo(theme_def: &ThemeDefinition) -> io::Result<()> {
    let theme = theme_def.to_theme();

    let _ = Input::new("Preview input")
        .description(
            "This uses the active theme for title, description, prompt, cursor, and placeholder.",
        )
        .placeholder("name@example.com")
        .prompt("Email: ")
        .theme(&theme)
        .run();

    let _ = Select::new("Preview select")
        .description("Single-choice widgets reuse the active cursor, option, and help styles.")
        .option(DemandOption::new("Compact").description("Small and dense"))
        .option(DemandOption::new("Comfortable").description("A little more room"))
        .option(DemandOption::new("Spacious").description("Lots of breathing room"))
        .theme(&theme)
        .run();

    let _ = MultiSelect::new("Preview multiselect")
        .description("Multi-select shows selected prefixes, help text, and error colors.")
        .min(1)
        .max(2)
        .option(DemandOption::new("Input"))
        .option(DemandOption::new("Select").selected(true))
        .option(DemandOption::new("Wizard"))
        .theme(&theme)
        .run();

    let _ = Confirm::new("Keep this theme active?")
        .description("This confirm prompt previews focused and blurred button styles.")
        .affirmative("Looks good")
        .negative("Keep editing")
        .theme(&theme)
        .run();

    pause(
        &theme,
        "Live demo complete",
        "Press enter to return to the editor.",
    )
}

fn select_active_theme(store: &mut ThemeStore) -> io::Result<()> {
    let active_theme = store.active_theme_definition().to_theme();
    let selected = Select::new("Select active theme")
        .description("Built-in and custom themes are both available here.")
        .options(store.theme_options(Some(&store.active_theme)))
        .theme(&active_theme)
        .run()?;

    store.active_theme = selected;
    store.save()
}

fn create_theme(store: &mut ThemeStore) -> io::Result<()> {
    let active_theme = store.active_theme_definition().to_theme();
    let base_name = Select::new("Choose a base theme")
        .description("New custom themes start from an existing theme.")
        .options(store.theme_options(None))
        .theme(&active_theme)
        .run()?;

    let base_theme = store
        .find_theme(&base_name)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "base theme not found"))?
        .clone();

    let mut draft = base_theme.clone();
    draft.built_in = false;
    draft.name = prompt_for_theme_name(&active_theme, store, None, "New theme name")?;

    edit_theme_loop(&mut draft, store)?;
    store.upsert_custom_theme(draft.clone());
    store.active_theme = draft.name;
    store.save()
}

fn edit_custom_theme(store: &mut ThemeStore) -> io::Result<()> {
    let custom_themes = store.custom_themes();
    if custom_themes.is_empty() {
        let active = store.active_theme_definition().to_theme();
        return pause(
            &active,
            "No custom themes yet",
            "Create a theme first, then come back here to edit it.",
        );
    }

    let active_theme = store.active_theme_definition().to_theme();
    let selected_name = Select::new("Edit custom theme")
        .description("Pick a saved custom theme to modify.")
        .options(store.custom_theme_options())
        .theme(&active_theme)
        .run()?;

    let existing = store
        .find_theme(&selected_name)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "theme not found"))?
        .clone();

    let mut draft = existing.clone();
    let old_name = draft.name.clone();
    edit_theme_loop(&mut draft, store)?;

    if old_name != draft.name {
        store.remove_custom_theme(&old_name);
        if store.active_theme == old_name {
            store.active_theme = draft.name.clone();
        }
    }

    store.upsert_custom_theme(draft);
    store.save()
}

fn delete_custom_theme(store: &mut ThemeStore) -> io::Result<()> {
    let custom_themes = store.custom_themes();
    if custom_themes.is_empty() {
        let active = store.active_theme_definition().to_theme();
        return pause(
            &active,
            "No custom themes to delete",
            "Only custom themes can be deleted.",
        );
    }

    let active_theme = store.active_theme_definition().to_theme();
    let selected_name = Select::new("Delete custom theme")
        .description("Choose the custom theme you want to remove.")
        .options(store.custom_theme_options())
        .theme(&active_theme)
        .run()?;

    let confirm = Confirm::new("Delete selected theme?")
        .description(&format!("This will permanently remove '{selected_name}'."))
        .affirmative("Delete")
        .negative("Keep")
        .theme(&active_theme)
        .run()?;

    if confirm {
        store.remove_custom_theme(&selected_name);
        if store.active_theme == selected_name {
            store.active_theme = String::from("charm");
        }
        store.save()?;
    }

    Ok(())
}

fn export_active_theme(store: &mut ThemeStore) -> io::Result<()> {
    let active = store.active_theme_definition().clone();
    let theme = active.to_theme();
    let default_path = default_export_path(&active.name)?;
    let export_path = prompt_for_path(
        &theme,
        "Export active theme",
        "Choose where to write this theme file.",
        &default_path.to_string_lossy(),
        false,
    )?;

    if export_path.exists() {
        let overwrite = Confirm::new("Overwrite existing file?")
            .description(&format!("{}", export_path.display()))
            .affirmative("Overwrite")
            .negative("Cancel")
            .theme(&theme)
            .run()?;
        if !overwrite {
            return Ok(());
        }
    }

    if let Some(parent) = export_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&export_path, active.to_export_document())?;
    pause(
        &theme,
        "Theme exported",
        &format!("Wrote {}", export_path.display()),
    )
}

fn import_theme(store: &mut ThemeStore) -> io::Result<()> {
    let active_theme = store.active_theme_definition().to_theme();
    let import_path = prompt_for_path(
        &active_theme,
        "Import theme from file",
        "Enter a path to a previously exported theme file.",
        "",
        true,
    )?;

    let contents = fs::read_to_string(&import_path)?;
    let mut imported = ThemeDefinition::from_export_document(&contents)?;
    imported.built_in = false;

    if store.find_theme(&imported.name).is_some() {
        imported.name = prompt_for_theme_name(
            &active_theme,
            store,
            None,
            "Imported theme name already exists. Choose a new name",
        )?;
    }

    store.upsert_custom_theme(imported.clone());
    store.active_theme = imported.name.clone();
    store.save()?;

    let imported_theme = imported.to_theme();
    pause(
        &imported_theme,
        "Theme imported",
        &format!("Loaded '{}' from {}", imported.name, import_path.display()),
    )
}

fn edit_theme_loop(draft: &mut ThemeDefinition, store: &ThemeStore) -> io::Result<()> {
    loop {
        let theme = draft.to_theme();
        let description = format!(
            "Editing '{}' • force cursor style={}",
            draft.name, draft.force_style
        );

        let action = Select::new("Edit custom theme")
            .description(&description)
            .option(
                DemandOption::with_label("Rename theme", EditAction::Rename)
                    .description("Change the stored name for this custom theme"),
            )
            .option(
                DemandOption::with_label("Edit text tokens", EditAction::EditTextTokens)
                    .description("Cursor strings, option prefixes, and breadcrumb separators"),
            )
            .option(
                DemandOption::with_label("Edit styles", EditAction::EditStyles)
                    .description("Per-role foreground, background, bold, and underline"),
            )
            .option(
                DemandOption::with_label(
                    "Toggle force cursor style",
                    EditAction::ToggleForceCursorStyle,
                )
                .description("Always use cursor_style instead of inheriting nearby text color"),
            )
            .option(
                DemandOption::with_label("Preview current palette", EditAction::PreviewPalette)
                    .description("Static readout of the current full theme definition"),
            )
            .option(
                DemandOption::with_label("Run live widget demo", EditAction::LiveDemo)
                    .description("Exercise widgets with the in-progress theme"),
            )
            .option(
                DemandOption::with_label("Save and return", EditAction::SaveAndReturn)
                    .description("Keep changes and go back to the advanced hub"),
            )
            .theme(&theme)
            .run()?;

        match action {
            EditAction::Rename => {
                draft.name = prompt_for_theme_name(&theme, store, Some(&draft.name), "Theme name")?;
            }
            EditAction::EditTextTokens => edit_text_tokens(draft)?,
            EditAction::EditStyles => edit_styles(draft)?,
            EditAction::ToggleForceCursorStyle => draft.force_style = !draft.force_style,
            EditAction::PreviewPalette => preview_palette(draft)?,
            EditAction::LiveDemo => run_live_demo(draft)?,
            EditAction::SaveAndReturn => break,
        }
    }

    Ok(())
}

fn edit_text_tokens(draft: &mut ThemeDefinition) -> io::Result<()> {
    loop {
        let theme = draft.to_theme();
        let field = Select::new("Edit text tokens")
            .description("These strings affect prefixes, cursors, and breadcrumb separators.")
            .option(DemandOption::with_label(
                format!("Cursor string [{}]", draft.cursor_str),
                Some(TextField::CursorStr),
            ))
            .option(DemandOption::with_label(
                format!("Selected prefix [{}]", draft.selected_prefix),
                Some(TextField::SelectedPrefix),
            ))
            .option(DemandOption::with_label(
                format!("Unselected prefix [{}]", draft.unselected_prefix),
                Some(TextField::UnselectedPrefix),
            ))
            .option(DemandOption::with_label(
                format!("Breadcrumb separator [{}]", draft.breadcrumb_separator),
                Some(TextField::BreadcrumbSeparator),
            ))
            .option(DemandOption::with_label("Back", None::<TextField>))
            .theme(&theme)
            .run()?;

        let Some(field) = field else {
            break;
        };

        match field {
            TextField::CursorStr => {
                draft.cursor_str = prompt_for_text(
                    &theme,
                    "Cursor string",
                    "A short symbol like ❯ or >",
                    &draft.cursor_str,
                )?;
            }
            TextField::SelectedPrefix => {
                draft.selected_prefix = prompt_for_text(
                    &theme,
                    "Selected prefix",
                    "Shown next to selected options.",
                    &draft.selected_prefix,
                )?;
            }
            TextField::UnselectedPrefix => {
                draft.unselected_prefix = prompt_for_text(
                    &theme,
                    "Unselected prefix",
                    "Shown next to unselected options.",
                    &draft.unselected_prefix,
                )?;
            }
            TextField::BreadcrumbSeparator => {
                draft.breadcrumb_separator = prompt_for_text(
                    &theme,
                    "Breadcrumb separator",
                    "Examples: > or -> or /",
                    &draft.breadcrumb_separator,
                )?;
            }
        }
    }

    Ok(())
}

fn edit_styles(draft: &mut ThemeDefinition) -> io::Result<()> {
    loop {
        let theme = draft.to_theme();
        let role = Select::new("Edit styles")
            .description("Pick a themed role to adjust its colors and text attributes.")
            .options(style_role_options(draft))
            .theme(&theme)
            .run()?;

        let Some(role) = role else {
            break;
        };

        edit_style_role(draft, role)?;
    }

    Ok(())
}

fn edit_style_role(draft: &mut ThemeDefinition, role: StyleRole) -> io::Result<()> {
    loop {
        let theme = draft.to_theme();
        let description = role.style_ref(draft).describe();

        let action = Select::new(role.label())
            .description(&description)
            .option(DemandOption::with_label(
                "Set foreground color",
                StyleAction::SetForeground,
            ))
            .option(DemandOption::with_label(
                "Clear foreground color",
                StyleAction::ClearForeground,
            ))
            .option(DemandOption::with_label(
                "Set background color",
                StyleAction::SetBackground,
            ))
            .option(DemandOption::with_label(
                "Clear background color",
                StyleAction::ClearBackground,
            ))
            .option(DemandOption::with_label(
                "Toggle bold",
                StyleAction::ToggleBold,
            ))
            .option(DemandOption::with_label(
                "Toggle underline",
                StyleAction::ToggleUnderline,
            ))
            .option(DemandOption::with_label("Back", StyleAction::Back))
            .theme(&theme)
            .run()?;

        match action {
            StyleAction::SetForeground => {
                let color = prompt_for_color(
                    &theme,
                    &format!("{} foreground", role.label()),
                    "Examples: red, white, 243, #fe8019, or 131,165,152",
                )?;
                role.style_mut(draft).fg = Some(color);
            }
            StyleAction::ClearForeground => role.style_mut(draft).fg = None,
            StyleAction::SetBackground => {
                let color = prompt_for_color(
                    &theme,
                    &format!("{} background", role.label()),
                    "Examples: black, 0, #282828, or 40,40,40",
                )?;
                role.style_mut(draft).bg = Some(color);
            }
            StyleAction::ClearBackground => role.style_mut(draft).bg = None,
            StyleAction::ToggleBold => {
                let style = role.style_mut(draft);
                style.bold = !style.bold;
            }
            StyleAction::ToggleUnderline => {
                let style = role.style_mut(draft);
                style.underline = !style.underline;
            }
            StyleAction::Back => break,
        }
    }

    Ok(())
}

fn render_theme_preview(theme_def: &ThemeDefinition) -> io::Result<String> {
    let theme = theme_def.to_theme();
    let mut out = Buffer::ansi();

    out.set_color(&theme.title)?;
    writeln!(out, "Demand Theme Preview")?;

    out.set_color(&theme.description)?;
    writeln!(
        out,
        "{} ({})",
        theme_def.name,
        if theme_def.built_in {
            "built-in"
        } else {
            "custom"
        }
    )?;
    writeln!(out)?;

    write_labeled_sample(&mut out, "Title", &theme.title, "Demand Theme Editor")?;
    write_labeled_sample(
        &mut out,
        "Description",
        &theme.description,
        "Supporting copy and helper text",
    )?;
    write_labeled_sample(&mut out, "Cursor", &theme.cursor, &theme.cursor_str)?;

    out.reset()?;
    write!(out, "Selected option: ")?;
    out.set_color(&theme.selected_prefix_fg)?;
    write!(out, "{}", theme.selected_prefix)?;
    out.set_color(&theme.selected_option)?;
    writeln!(out, " Preview selection")?;

    out.reset()?;
    write!(out, "Unselected option: ")?;
    out.set_color(&theme.unselected_prefix_fg)?;
    write!(out, "{}", theme.unselected_prefix)?;
    out.set_color(&theme.unselected_option)?;
    writeln!(out, " Preview item")?;

    out.reset()?;
    write!(out, "Input: ")?;
    out.set_color(&theme.input_prompt)?;
    write!(out, "Email: ")?;
    out.set_color(&theme.real_cursor_color(Some(&theme.input_placeholder)))?;
    write!(out, "n")?;
    out.set_color(&theme.input_placeholder)?;
    writeln!(out, "ame@example.com")?;

    out.reset()?;
    write!(out, "Help: ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "enter")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " confirm ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "esc")?;
    out.set_color(&theme.help_desc)?;
    writeln!(out, " cancel")?;

    out.reset()?;
    write!(out, "Buttons: ")?;
    out.set_color(&theme.focused_button)?;
    write!(out, "  Save  ")?;
    out.reset()?;
    write!(out, " ")?;
    out.set_color(&theme.blurred_button)?;
    writeln!(out, "  Cancel  ")?;

    out.reset()?;
    write!(out, "Error: ")?;
    out.set_color(&theme.error_indicator)?;
    writeln!(out, "✗ Validation failed")?;

    out.reset()?;
    write!(out, "Breadcrumb: ")?;
    out.set_color(&theme.breadcrumb_clickable)?;
    write!(out, "1:Themes")?;
    out.set_color(&theme.description)?;
    write!(out, "{}", theme.breadcrumb_separator)?;
    out.set_color(&theme.breadcrumb_active)?;
    write!(out, "[2:Preview]")?;
    out.set_color(&theme.description)?;
    write!(out, "{}", theme.breadcrumb_separator)?;
    out.set_color(&theme.breadcrumb_future)?;
    writeln!(out, "3:Save")?;

    out.reset()?;
    writeln!(out)?;
    writeln!(out, "Force cursor style: {}", theme_def.force_style)?;
    writeln!(out)?;
    writeln!(out, "Text tokens:")?;
    writeln!(out, "  cursor_str={}", theme_def.cursor_str)?;
    writeln!(out, "  selected_prefix={}", theme_def.selected_prefix)?;
    writeln!(out, "  unselected_prefix={}", theme_def.unselected_prefix)?;
    writeln!(
        out,
        "  breadcrumb_separator={}",
        theme_def.breadcrumb_separator
    )?;
    writeln!(out)?;
    writeln!(out, "Role breakdown:")?;
    for role in StyleRole::all() {
        write_role_breakdown(&mut out, role, role.style_ref(theme_def))?;
    }

    out.reset()?;
    Ok(String::from_utf8_lossy(out.as_slice()).into_owned())
}

fn write_labeled_sample(
    out: &mut Buffer,
    label: &str,
    style: &ColorSpec,
    sample: &str,
) -> io::Result<()> {
    out.reset()?;
    write!(out, "{label}: ")?;
    out.set_color(style)?;
    writeln!(out, "{sample}")?;
    out.reset()?;
    Ok(())
}

fn write_role_breakdown(out: &mut Buffer, role: StyleRole, style: &StoredStyle) -> io::Result<()> {
    out.reset()?;
    write!(out, "  {}: ", role.label())?;
    out.set_color(&style.to_spec())?;
    write!(out, "Sample")?;
    out.reset()?;
    writeln!(out, " [{}]", style.describe())?;
    Ok(())
}

fn style_role_options(theme: &ThemeDefinition) -> Vec<DemandOption<Option<StyleRole>>> {
    let mut options = Vec::new();
    for role in StyleRole::all() {
        let style = role.style_ref(theme);
        options.push(DemandOption::with_label(
            format!("{} [{}]", role.label(), style.describe()),
            Some(role),
        ));
    }
    options.push(DemandOption::with_label("Back", None));
    options
}

impl StyleRole {
    fn all() -> [StyleRole; 20] {
        [
            StyleRole::Title,
            StyleRole::Description,
            StyleRole::Cursor,
            StyleRole::SelectedPrefix,
            StyleRole::SelectedOption,
            StyleRole::UnselectedPrefix,
            StyleRole::UnselectedOption,
            StyleRole::CursorStyle,
            StyleRole::InputCursor,
            StyleRole::InputPlaceholder,
            StyleRole::InputPrompt,
            StyleRole::HelpKey,
            StyleRole::HelpDesc,
            StyleRole::HelpSep,
            StyleRole::FocusedButton,
            StyleRole::BlurredButton,
            StyleRole::ErrorIndicator,
            StyleRole::BreadcrumbActive,
            StyleRole::BreadcrumbClickable,
            StyleRole::BreadcrumbFuture,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            StyleRole::Title => "Title",
            StyleRole::Description => "Description",
            StyleRole::Cursor => "Cursor",
            StyleRole::SelectedPrefix => "Selected prefix",
            StyleRole::SelectedOption => "Selected option",
            StyleRole::UnselectedPrefix => "Unselected prefix",
            StyleRole::UnselectedOption => "Unselected option",
            StyleRole::CursorStyle => "Cursor fallback",
            StyleRole::InputCursor => "Input cursor",
            StyleRole::InputPlaceholder => "Input placeholder",
            StyleRole::InputPrompt => "Input prompt",
            StyleRole::HelpKey => "Help key",
            StyleRole::HelpDesc => "Help description",
            StyleRole::HelpSep => "Help separator",
            StyleRole::FocusedButton => "Focused button",
            StyleRole::BlurredButton => "Blurred button",
            StyleRole::ErrorIndicator => "Error indicator",
            StyleRole::BreadcrumbActive => "Breadcrumb active",
            StyleRole::BreadcrumbClickable => "Breadcrumb clickable",
            StyleRole::BreadcrumbFuture => "Breadcrumb future",
        }
    }

    fn style_ref(self, theme: &ThemeDefinition) -> &StoredStyle {
        match self {
            StyleRole::Title => &theme.title,
            StyleRole::Description => &theme.description,
            StyleRole::Cursor => &theme.cursor,
            StyleRole::SelectedPrefix => &theme.selected_prefix_fg,
            StyleRole::SelectedOption => &theme.selected_option,
            StyleRole::UnselectedPrefix => &theme.unselected_prefix_fg,
            StyleRole::UnselectedOption => &theme.unselected_option,
            StyleRole::CursorStyle => &theme.cursor_style,
            StyleRole::InputCursor => &theme.input_cursor,
            StyleRole::InputPlaceholder => &theme.input_placeholder,
            StyleRole::InputPrompt => &theme.input_prompt,
            StyleRole::HelpKey => &theme.help_key,
            StyleRole::HelpDesc => &theme.help_desc,
            StyleRole::HelpSep => &theme.help_sep,
            StyleRole::FocusedButton => &theme.focused_button,
            StyleRole::BlurredButton => &theme.blurred_button,
            StyleRole::ErrorIndicator => &theme.error_indicator,
            StyleRole::BreadcrumbActive => &theme.breadcrumb_active,
            StyleRole::BreadcrumbClickable => &theme.breadcrumb_clickable,
            StyleRole::BreadcrumbFuture => &theme.breadcrumb_future,
        }
    }

    fn style_mut(self, theme: &mut ThemeDefinition) -> &mut StoredStyle {
        match self {
            StyleRole::Title => &mut theme.title,
            StyleRole::Description => &mut theme.description,
            StyleRole::Cursor => &mut theme.cursor,
            StyleRole::SelectedPrefix => &mut theme.selected_prefix_fg,
            StyleRole::SelectedOption => &mut theme.selected_option,
            StyleRole::UnselectedPrefix => &mut theme.unselected_prefix_fg,
            StyleRole::UnselectedOption => &mut theme.unselected_option,
            StyleRole::CursorStyle => &mut theme.cursor_style,
            StyleRole::InputCursor => &mut theme.input_cursor,
            StyleRole::InputPlaceholder => &mut theme.input_placeholder,
            StyleRole::InputPrompt => &mut theme.input_prompt,
            StyleRole::HelpKey => &mut theme.help_key,
            StyleRole::HelpDesc => &mut theme.help_desc,
            StyleRole::HelpSep => &mut theme.help_sep,
            StyleRole::FocusedButton => &mut theme.focused_button,
            StyleRole::BlurredButton => &mut theme.blurred_button,
            StyleRole::ErrorIndicator => &mut theme.error_indicator,
            StyleRole::BreadcrumbActive => &mut theme.breadcrumb_active,
            StyleRole::BreadcrumbClickable => &mut theme.breadcrumb_clickable,
            StyleRole::BreadcrumbFuture => &mut theme.breadcrumb_future,
        }
    }
}

impl ThemeStore {
    fn theme_options(&self, selected: Option<&str>) -> Vec<DemandOption<String>> {
        self.themes
            .iter()
            .map(|theme| {
                let mut option = DemandOption::with_label(
                    format!(
                        "{} ({})",
                        theme.name,
                        if theme.built_in { "built-in" } else { "custom" }
                    ),
                    theme.name.clone(),
                );
                if selected == Some(theme.name.as_str()) {
                    option = option.selected(true);
                }
                option
            })
            .collect()
    }

    fn custom_theme_options(&self) -> Vec<DemandOption<String>> {
        self.themes
            .iter()
            .filter(|theme| !theme.built_in)
            .map(|theme| DemandOption::with_label(theme.name.clone(), theme.name.clone()))
            .collect()
    }
}

fn prompt_for_theme_name(
    theme: &Theme,
    store: &ThemeStore,
    exclude: Option<&str>,
    title: &str,
) -> io::Result<String> {
    Input::new(title)
        .description("Theme names must be unique and non-empty.")
        .theme(theme)
        .validator({
            let exclude = exclude.map(|value| value.to_string());
            let names = store
                .themes
                .iter()
                .map(|theme| theme.name.clone())
                .collect::<Vec<_>>();
            move |value: &str| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(String::from("Theme name cannot be empty"));
                }
                let already_exists = names.iter().any(|name| {
                    name == trimmed
                        && exclude
                            .as_ref()
                            .map(|excluded| excluded != trimmed)
                            .unwrap_or(true)
                });
                if already_exists {
                    return Err(String::from("Theme name already exists"));
                }
                Ok(())
            }
        })
        .run()
        .map(|value| value.trim().to_string())
}

fn prompt_for_text(
    theme: &Theme,
    title: &str,
    description: &str,
    default_value: &str,
) -> io::Result<String> {
    Input::new(title)
        .description(description)
        .default_value(default_value.to_string())
        .theme(theme)
        .run()
}

fn prompt_for_color(theme: &Theme, title: &str, description: &str) -> io::Result<StoredColor> {
    Input::new(title)
        .description(description)
        .theme(theme)
        .validator(|value: &str| match Color::from_str(value.trim()) {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("Invalid color: {err}")),
        })
        .run()
        .and_then(|value| parse_user_color(&value))
}

fn prompt_for_color_with_default(
    theme: &Theme,
    title: &str,
    description: &str,
    default_value: &str,
) -> io::Result<StoredColor> {
    Input::new(title)
        .description(description)
        .default_value(default_value.to_string())
        .theme(theme)
        .validator(|value: &str| match Color::from_str(value.trim()) {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("Invalid color: {err}")),
        })
        .run()
        .and_then(|value| parse_user_color(&value))
}

fn pick_color_with_picker(
    theme: &Theme,
    title: &str,
    description: &str,
    initial: StoredColor,
) -> io::Result<Option<StoredColor>> {
    let mut term = Term::stderr();
    let presets = preset_palette_rows();
    let (mut focus_row, mut focus_col) = nearest_preset_cell(initial);
    let mut fine_tune_mode = false;
    let mut fine_tune_channel = 0usize;
    let mut current = initial;

    term.hide_cursor()?;
    let result = (|| -> io::Result<Option<StoredColor>> {
        loop {
            let output = render_color_picker(
                theme,
                title,
                description,
                current,
                focus_row,
                focus_col,
                fine_tune_mode,
                fine_tune_channel,
            )?;
            term.clear_screen()?;
            term.write_all(output.as_bytes())?;
            term.flush()?;

            match term.read_key()? {
                Key::ArrowLeft | Key::Char('h') => {
                    if fine_tune_mode {
                        current = adjust_color_channel(current, fine_tune_channel, -8);
                    } else if focus_row < presets.len() {
                        if focus_col > 0 {
                            focus_col -= 1;
                        }
                        current = presets[focus_row].1[focus_col];
                    }
                }
                Key::ArrowRight | Key::Char('l') => {
                    if fine_tune_mode {
                        current = adjust_color_channel(current, fine_tune_channel, 8);
                    } else if focus_row < presets.len() {
                        if focus_col + 1 < presets[focus_row].1.len() {
                            focus_col += 1;
                        }
                        current = presets[focus_row].1[focus_col];
                    }
                }
                Key::ArrowUp | Key::Char('k') => {
                    if fine_tune_mode {
                        fine_tune_channel = fine_tune_channel.saturating_sub(1);
                    } else if focus_row > 0 {
                        focus_row -= 1;
                        if focus_row < presets.len() {
                            focus_col = focus_col.min(presets[focus_row].1.len() - 1);
                            current = presets[focus_row].1[focus_col];
                        }
                    }
                }
                Key::ArrowDown | Key::Char('j') => {
                    if fine_tune_mode {
                        if fine_tune_channel < 2 {
                            fine_tune_channel += 1;
                        }
                    } else if focus_row + 1 < presets.len() {
                        focus_row += 1;
                        focus_col = focus_col.min(presets[focus_row].1.len() - 1);
                        current = presets[focus_row].1[focus_col];
                    }
                }
                Key::Char(' ') => {
                    fine_tune_mode = !fine_tune_mode;
                }
                Key::Enter => {
                    break Ok(Some(current));
                }
                Key::Char('t') => {
                    term.show_cursor()?;
                    match prompt_for_color_with_default(
                        theme,
                        title,
                        "Type an exact color value like red, 243, #fe8019, or 131,165,152",
                        &current.display_value(),
                    ) {
                        Ok(color) => {
                            let (row, col) = nearest_preset_cell(color);
                            focus_row = row;
                            focus_col = col;
                            current = color;
                            term.hide_cursor()?;
                        }
                        Err(err) if err.kind() == io::ErrorKind::Interrupted => {
                            term.hide_cursor()?;
                        }
                        Err(err) => {
                            break Err(err);
                        }
                    }
                }
                Key::Char('r') => {
                    let (row, col) = nearest_preset_cell(initial);
                    focus_row = row;
                    focus_col = col;
                    fine_tune_channel = 0;
                    current = initial;
                }
                Key::Escape | Key::Char('q') => {
                    break Ok(None);
                }
                _ => {}
            }
        }
    })();

    term.show_cursor()?;
    result
}

fn parse_user_color(value: &str) -> io::Result<StoredColor> {
    let color = Color::from_str(value.trim()).map_err(|err| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid color: {err}"))
    })?;
    Ok(StoredColor::from_termcolor(&color))
}

fn render_color_picker(
    theme: &Theme,
    title: &str,
    description: &str,
    selected: StoredColor,
    focus_row: usize,
    focus_col: usize,
    fine_tune_mode: bool,
    fine_tune_channel: usize,
) -> io::Result<String> {
    let mut out = Buffer::ansi();
    let (r, g, b) = selected.rgb_triplet();
    let presets = preset_palette_rows();

    out.set_color(&theme.title)?;
    writeln!(out, "{title}")?;
    out.set_color(&theme.description)?;
    writeln!(out, "{description}")?;
    writeln!(out)?;

    out.reset()?;
    write!(out, "Selected: ")?;
    let swatch = swatch_style(selected);
    out.set_color(&swatch)?;
    write!(out, "       ")?;
    out.reset()?;
    writeln!(
        out,
        "  {}  nearest-ansi:{}  rgb:{},{},{}",
        selected.display_value(),
        selected.ansi_seed(),
        r,
        g,
        b
    )?;
    writeln!(out)?;

    out.set_color(&theme.description)?;
    writeln!(out, "Shade board")?;
    for (row, (_, colors)) in presets.iter().enumerate() {
        render_color_chip_row(&mut out, colors, focus_row == row, focus_col)?;
    }
    writeln!(out)?;

    out.set_color(&theme.description)?;
    writeln!(out, "Fine tune RGB")?;
    out.set_color(&theme.help_desc)?;
    writeln!(
        out,
        "Mode: {}",
        if fine_tune_mode {
            "fine tune"
        } else {
            "shade board"
        }
    )?;
    render_color_slider(&mut out, "R", r, fine_tune_mode && fine_tune_channel == 0)?;
    render_color_slider(&mut out, "G", g, fine_tune_mode && fine_tune_channel == 1)?;
    render_color_slider(&mut out, "B", b, fine_tune_mode && fine_tune_channel == 2)?;
    writeln!(out)?;

    out.set_color(&theme.help_key)?;
    write!(out, "←/→/↑/↓")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " move swatch or tweak channel ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "space")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " toggle board/fine tune ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "enter")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " confirm ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "t")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " type exact value ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "r")?;
    out.set_color(&theme.help_desc)?;
    write!(out, " reset ")?;
    out.set_color(&theme.help_sep)?;
    write!(out, "• ")?;
    out.set_color(&theme.help_key)?;
    write!(out, "q/esc")?;
    out.set_color(&theme.help_desc)?;
    writeln!(out, " cancel")?;

    out.reset()?;
    Ok(String::from_utf8_lossy(out.as_slice()).into_owned())
}

fn render_color_chip_row(
    out: &mut Buffer,
    colors: &[StoredColor; 8],
    focused_row: bool,
    focused_col: usize,
) -> io::Result<()> {
    for _ in 0..2 {
        out.reset()?;
        for (index, color) in colors.iter().enumerate() {
            let style = shade_cell_style(*color);
            out.set_color(&style)?;
            if focused_row && index == focused_col {
                write!(out, "▓▓")?;
            } else {
                write!(out, "██")?;
            }
        }
        out.reset()?;
        writeln!(out)?;
    }
    Ok(())
}

fn render_color_slider(out: &mut Buffer, label: &str, value: u8, focused: bool) -> io::Result<()> {
    out.reset()?;
    if focused {
        write!(out, "> ")?;
    } else {
        write!(out, "  ")?;
    }
    write!(out, "{label} ")?;
    let filled = usize::from(value) * 24 / 255;
    write!(out, "[")?;
    for index in 0..24 {
        if index < filled {
            write!(out, "█")?;
        } else {
            write!(out, "░")?;
        }
    }
    writeln!(out, "] {:>3}", value)?;
    Ok(())
}

fn prompt_for_path(
    theme: &Theme,
    title: &str,
    description: &str,
    default_value: &str,
    must_exist: bool,
) -> io::Result<PathBuf> {
    Input::new(title)
        .description(description)
        .default_value(default_value.to_string())
        .theme(theme)
        .validator(move |value: &str| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(String::from("Path cannot be empty"));
            }
            let path = PathBuf::from(trimmed);
            if must_exist && !path.exists() {
                return Err(String::from("Path does not exist"));
            }
            Ok(())
        })
        .run()
        .map(|value| PathBuf::from(value.trim()))
}

fn pause(theme: &Theme, title: &str, description: &str) -> io::Result<()> {
    let _ = Input::new(title)
        .description(description)
        .prompt("")
        .theme(theme)
        .run()?;
    Ok(())
}

fn store_path() -> io::Result<PathBuf> {
    let base = if let Ok(path) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(path)
    } else if let Ok(path) = env::var("APPDATA") {
        PathBuf::from(path)
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        env::current_dir()?
    };

    Ok(base.join("demand").join(STORE_FILE))
}

fn default_export_path(theme_name: &str) -> io::Result<PathBuf> {
    let mut path = store_path()?;
    path.pop();
    path.push("exports");
    path.push(format!("{}.demand-theme", sanitize_filename(theme_name)));
    Ok(path)
}

fn sanitize_filename(input: &str) -> String {
    let cleaned = input
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '-',
        })
        .collect::<String>();

    if cleaned.is_empty() {
        String::from("theme")
    } else {
        cleaned
    }
}

fn escape(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn unescape(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

fn parse_bool(value: &str) -> io::Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid bool '{value}'"),
        )),
    }
}

fn bool_flag(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn invalid_rgb(input: &str) -> impl Fn(std::num::ParseIntError) -> io::Error + '_ {
    move |err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid rgb component in '{input}': {err}"),
        )
    }
}

fn ansi256_rgb(value: u8) -> (u8, u8, u8) {
    if value < 16 {
        return match value {
            0 => (0, 0, 0),
            1 => (128, 0, 0),
            2 => (0, 128, 0),
            3 => (128, 128, 0),
            4 => (0, 0, 128),
            5 => (128, 0, 128),
            6 => (0, 128, 128),
            7 => (192, 192, 192),
            8 => (128, 128, 128),
            9 => (255, 0, 0),
            10 => (0, 255, 0),
            11 => (255, 255, 0),
            12 => (0, 0, 255),
            13 => (255, 0, 255),
            14 => (0, 255, 255),
            _ => (255, 255, 255),
        };
    }

    if value >= 232 {
        let shade = 8 + (value - 232) * 10;
        return (shade, shade, shade);
    }

    let index = value - 16;
    let r = index / 36;
    let g = (index % 36) / 6;
    let b = index % 6;
    let step = |component: u8| match component {
        0 => 0,
        _ => 55 + component * 40,
    };
    (step(r), step(g), step(b))
}

fn nearest_ansi256(r: u8, g: u8, b: u8) -> u8 {
    let mut best = 0u8;
    let mut best_distance = u32::MAX;

    for index in 0..=255u8 {
        let (cr, cg, cb) = ansi256_rgb(index);
        let dr = i32::from(r) - i32::from(cr);
        let dg = i32::from(g) - i32::from(cg);
        let db = i32::from(b) - i32::from(cb);
        let distance = (dr * dr + dg * dg + db * db) as u32;
        if distance < best_distance {
            best_distance = distance;
            best = index;
        }
    }

    best
}

fn preset_palette_rows() -> [(&'static str, [StoredColor; 8]); 8] {
    [
        (
            "Gray",
            [
                StoredColor::Rgb(255, 255, 255),
                StoredColor::Rgb(216, 216, 216),
                StoredColor::Rgb(176, 176, 176),
                StoredColor::Rgb(136, 136, 136),
                StoredColor::Rgb(104, 104, 104),
                StoredColor::Rgb(72, 72, 72),
                StoredColor::Rgb(40, 40, 40),
                StoredColor::Rgb(0, 0, 0),
            ],
        ),
        (
            "Red",
            [
                StoredColor::Rgb(254, 205, 211),
                StoredColor::Rgb(252, 165, 165),
                StoredColor::Rgb(248, 113, 113),
                StoredColor::Rgb(239, 68, 68),
                StoredColor::Rgb(220, 38, 38),
                StoredColor::Rgb(185, 28, 28),
                StoredColor::Rgb(153, 27, 27),
                StoredColor::Rgb(127, 29, 29),
            ],
        ),
        (
            "Orange",
            [
                StoredColor::Rgb(254, 215, 170),
                StoredColor::Rgb(253, 186, 116),
                StoredColor::Rgb(251, 146, 60),
                StoredColor::Rgb(249, 115, 22),
                StoredColor::Rgb(234, 88, 12),
                StoredColor::Rgb(194, 65, 12),
                StoredColor::Rgb(154, 52, 18),
                StoredColor::Rgb(124, 45, 18),
            ],
        ),
        (
            "Yellow",
            [
                StoredColor::Rgb(254, 240, 138),
                StoredColor::Rgb(253, 224, 71),
                StoredColor::Rgb(250, 204, 21),
                StoredColor::Rgb(234, 179, 8),
                StoredColor::Rgb(202, 138, 4),
                StoredColor::Rgb(161, 98, 7),
                StoredColor::Rgb(133, 77, 14),
                StoredColor::Rgb(113, 63, 18),
            ],
        ),
        (
            "Green",
            [
                StoredColor::Rgb(187, 247, 208),
                StoredColor::Rgb(134, 239, 172),
                StoredColor::Rgb(74, 222, 128),
                StoredColor::Rgb(34, 197, 94),
                StoredColor::Rgb(22, 163, 74),
                StoredColor::Rgb(21, 128, 61),
                StoredColor::Rgb(22, 101, 52),
                StoredColor::Rgb(20, 83, 45),
            ],
        ),
        (
            "Blue",
            [
                StoredColor::Rgb(191, 219, 254),
                StoredColor::Rgb(147, 197, 253),
                StoredColor::Rgb(96, 165, 250),
                StoredColor::Rgb(59, 130, 246),
                StoredColor::Rgb(37, 99, 235),
                StoredColor::Rgb(29, 78, 216),
                StoredColor::Rgb(30, 64, 175),
                StoredColor::Rgb(30, 58, 138),
            ],
        ),
        (
            "Indigo",
            [
                StoredColor::Rgb(199, 210, 254),
                StoredColor::Rgb(165, 180, 252),
                StoredColor::Rgb(129, 140, 248),
                StoredColor::Rgb(99, 102, 241),
                StoredColor::Rgb(79, 70, 229),
                StoredColor::Rgb(67, 56, 202),
                StoredColor::Rgb(55, 48, 163),
                StoredColor::Rgb(49, 46, 129),
            ],
        ),
        (
            "Violet",
            [
                StoredColor::Rgb(221, 214, 254),
                StoredColor::Rgb(196, 181, 253),
                StoredColor::Rgb(167, 139, 250),
                StoredColor::Rgb(139, 92, 246),
                StoredColor::Rgb(124, 58, 237),
                StoredColor::Rgb(109, 40, 217),
                StoredColor::Rgb(91, 33, 182),
                StoredColor::Rgb(76, 29, 149),
            ],
        ),
    ]
}

fn nearest_preset_cell(color: StoredColor) -> (usize, usize) {
    let presets = preset_palette_rows();
    let (r, g, b) = color.rgb_triplet();
    let mut best = (0usize, 0usize);
    let mut best_distance = u32::MAX;

    for (row, (_, colors)) in presets.iter().enumerate() {
        for (col, preset) in colors.iter().enumerate() {
            let (pr, pg, pb) = preset.rgb_triplet();
            let dr = i32::from(r) - i32::from(pr);
            let dg = i32::from(g) - i32::from(pg);
            let db = i32::from(b) - i32::from(pb);
            let distance = (dr * dr + dg * dg + db * db) as u32;
            if distance < best_distance {
                best_distance = distance;
                best = (row, col);
            }
        }
    }

    best
}

fn adjust_color_channel(color: StoredColor, channel: usize, delta: i16) -> StoredColor {
    let (mut r, mut g, mut b) = color.rgb_triplet();
    match channel {
        0 => r = adjust_channel_value(r, delta),
        1 => g = adjust_channel_value(g, delta),
        _ => b = adjust_channel_value(b, delta),
    }
    StoredColor::Rgb(r, g, b)
}

fn adjust_channel_value(value: u8, delta: i16) -> u8 {
    let next = i16::from(value) + delta;
    next.clamp(0, 255) as u8
}
