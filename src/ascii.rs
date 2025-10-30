use crate::terminal::colors::ColorSystem;
use include_dir::{include_dir, Dir};

static LOGOS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/logos");

pub fn process_logo_colors(logo: &str, colors: &ColorSystem) -> String {
    if !colors.enabled() {
        return logo.to_string();
    }

    let mut result = String::with_capacity(logo.len() + 32);
    let mut chars = logo.chars().peekable();
    let mut current_color = colors.logo_slot(0);

    if !current_color.is_empty() {
        result.push_str(current_color);
    }

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(digit) = chars.peek().and_then(|d| d.to_digit(10)) {
                chars.next();
                current_color = colors.logo_slot(digit as usize);
                if !current_color.is_empty() {
                    result.push_str(current_color);
                }
                continue;
            }
        }

        result.push(ch);

        if ch == '\n' && !current_color.is_empty() {
            result.push_str(current_color);
        }
    }

    result.push_str(colors.reset());
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
