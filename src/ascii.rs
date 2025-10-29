use crate::config::Config;
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use include_dir::{include_dir, Dir};

static LOGOS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/logos");

// Helper to parse a color array from config once
fn parse_color_array(config: &Config) -> Vec<Color> {
    config
        .logo_color
        .as_ref()
        .map(|arr| {
            arr.split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .filter_map(|c| super::ui::parse_hex_color(c))
                .collect()
        })
        .unwrap_or_default()
}

fn get_logo_color(idx: usize, colors: &[Color]) -> Color {
    if idx > 0 && idx <= colors.len() {
        colors[idx - 1]
    } else {
        Color::White
    }
}

pub fn process_logo_colors(logo: &str, config: &Config) -> String {
    // Parse color array once instead of on every color code
    let colors = parse_color_array(config);

    let mut result = String::new();
    let mut chars = logo.chars().peekable();
    let mut current_color = SetForegroundColor(Color::White).to_string();

    while let Some(c) = chars.next() {
        if c == '$' {
            // Check for a digit after $
            if let Some(digit) = chars.peek().and_then(|d| d.to_digit(10)) {
                chars.next(); // consume digit
                let color = get_logo_color(digit as usize, &colors);
                current_color = format!("{}", SetForegroundColor(color));
                result.push_str(&current_color);
                continue;
            }
        }
        result.push(c);
        // If we hit a newline, also push the current color at the start of the next line
        if c == '\n' {
            result.push_str(&current_color);
        }
    }
    result.push_str(&format!("{}", ResetColor));
    result
}

pub fn get_ascii_logo(distro_id: &str) -> String {
    let filename = format!("{}.txt", distro_id);
    if let Some(file) = LOGOS_DIR.get_file(&filename) {
        file.contents_utf8().unwrap_or("Logo not found").to_string()
    } else {
        "Logo not found".to_string()
    }
}
