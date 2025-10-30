use crate::config::Config;

use super::logo_colors::{LogoColorEntry, LOGO_COLOR_ENTRIES};
use super::theme::TerminalTheme;

const RESET_SEQUENCE: &str = "\x1b[0m";
const MAX_LOGO_COLORS: usize = 9;

fn wrap_code(code: &str) -> String {
    if code.is_empty() {
        return String::new();
    }
    let mut wrapped = String::with_capacity(code.len() + 3);
    wrapped.push('\u{1b}');
    wrapped.push('[');
    wrapped.push_str(code);
    wrapped.push('m');
    wrapped
}

fn normalize(text: &str) -> String {
    text.trim().to_lowercase()
}

pub fn find_logo_colors(name: &str) -> Option<&'static LogoColorEntry> {
    if name.is_empty() {
        return None;
    }
    let query = normalize(name);
    if query.is_empty() {
        return None;
    }
    LOGO_COLOR_ENTRIES
        .iter()
        .find(|entry| entry.names.iter().any(|candidate| *candidate == query))
}

#[derive(Debug, Clone)]
pub struct ColorSystem {
    enabled: bool,
    logo_colors: Vec<String>,
    key_color: String,
    title_color: String,
    value_color: String,
    box_color: String,
}

impl ColorSystem {
    pub fn new(
        distro_id: &str,
        distro_name: &str,
        config: &Config,
        theme: Option<&TerminalTheme>,
        stdout_is_tty: bool,
    ) -> Self {
        let enabled = stdout_is_tty;

        let _ = theme;

        if !enabled {
            return Self {
                enabled: false,
                logo_colors: Vec::new(),
                key_color: String::new(),
                title_color: String::new(),
                value_color: String::new(),
                box_color: String::new(),
            };
        }

        let entry = find_logo_colors(distro_id)
            .or_else(|| find_logo_colors(distro_name))
            .or_else(|| find_logo_colors("unknown"))
            .unwrap_or_else(|| {
                LOGO_COLOR_ENTRIES
                    .first()
                    .expect("logo color dataset is empty")
            });

        let mut resolved_codes: Vec<String> = entry
            .colors
            .iter()
            .take(MAX_LOGO_COLORS)
            .map(|code| (*code).to_string())
            .collect();

        if resolved_codes.is_empty() {
            resolved_codes.push("39".to_string());
        }

        if let Some(spec) = config.logo_color.as_deref() {
            let overrides = parse_color_list(spec, MAX_LOGO_COLORS);
            if !overrides.is_empty() {
                resolved_codes = overrides;
            }
        }

        let logo_colors: Vec<String> = resolved_codes.iter().map(|code| wrap_code(code)).collect();

        let mut key_code = entry
            .color_keys
            .map(|code| code.to_string())
            .or_else(|| resolved_codes.get(1).cloned())
            .or_else(|| resolved_codes.get(0).cloned());

        if let Some(spec) = config.color_keys.as_deref() {
            if let Some(parsed) = parse_color_spec(spec) {
                key_code = Some(parsed);
            }
        }

        let mut title_code = entry
            .color_title
            .map(|code| code.to_string())
            .or_else(|| resolved_codes.get(0).cloned());

        if let Some(spec) = config.color_title.as_deref() {
            if let Some(parsed) = parse_color_spec(spec) {
                title_code = Some(parsed);
            }
        }

        let value_code = config.color.as_deref().and_then(parse_color_spec);

        let mut key_color = key_code
            .as_deref()
            .map(wrap_code)
            .unwrap_or_else(|| logo_colors.get(0).cloned().unwrap_or_default());

        if key_color.is_empty() {
            key_color = logo_colors.get(0).cloned().unwrap_or_default();
        }

        let mut title_color = title_code
            .as_deref()
            .map(wrap_code)
            .unwrap_or_else(|| logo_colors.get(0).cloned().unwrap_or_default());

        if title_color.is_empty() {
            title_color = logo_colors.get(0).cloned().unwrap_or_default();
        }

        let value_color = value_code.as_deref().map(wrap_code).unwrap_or_default();

        let box_color = config
            .box_outline_color
            .as_deref()
            .and_then(parse_color_spec)
            .map(|code| wrap_code(&code))
            .unwrap_or_default();

        Self {
            enabled,
            logo_colors,
            key_color,
            title_color,
            value_color,
            box_color,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn reset(&self) -> &'static str {
        if self.enabled {
            RESET_SEQUENCE
        } else {
            ""
        }
    }

    pub fn logo_slot(&self, slot: usize) -> &str {
        if !self.enabled {
            return "";
        }
        if slot == 0 {
            return &self.logo_colors[0];
        }
        self.logo_colors
            .get(slot.saturating_sub(1))
            .unwrap_or(&self.logo_colors[0])
    }

    pub fn key_color(&self) -> &str {
        if self.enabled {
            &self.key_color
        } else {
            ""
        }
    }

    pub fn title_color(&self) -> &str {
        if self.enabled {
            &self.title_color
        } else {
            ""
        }
    }

    pub fn value_color(&self) -> &str {
        if self.enabled {
            &self.value_color
        } else {
            ""
        }
    }

    pub fn box_color(&self) -> &str {
        if self.enabled {
            &self.box_color
        } else {
            ""
        }
    }
}

fn parse_color_list(spec: &str, limit: usize) -> Vec<String> {
    spec.split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|token| parse_color_spec(token))
        .take(limit)
        .collect()
}

fn parse_color_spec(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    for token in trimmed
        .split(|c: char| c.is_whitespace() || c == '+')
        .filter(|t| !t.is_empty())
    {
        segments.push(parse_color_token(token)?);
    }

    if segments.is_empty() {
        return None;
    }

    let mut combined = String::new();
    for segment in segments {
        if !combined.is_empty() {
            combined.push(';');
        }
        combined.push_str(&segment);
    }
    Some(combined)
}

fn parse_color_token(token: &str) -> Option<String> {
    let lower = token.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }

    if lower.starts_with("rgb:") {
        return parse_rgb_values(&lower[4..]);
    }

    if lower.starts_with("rgb(") && lower.ends_with(')') {
        return parse_rgb_values(&lower[4..lower.len() - 1]);
    }

    if let Some(hex) = lower.strip_prefix('#') {
        return parse_hex_color(hex).map(|(r, g, b)| format!("38;2;{r};{g};{b}"));
    }

    if let Some(rest) = lower.strip_prefix("256:") {
        return parse_palette_index(rest).map(|idx| format!("38;5;{idx}"));
    }

    if let Some(rest) = lower.strip_prefix("color256:") {
        return parse_palette_index(rest).map(|idx| format!("38;5;{idx}"));
    }

    if lower.contains(';') && lower.chars().all(|c| c.is_ascii_digit() || c == ';') {
        return Some(lower);
    }

    if lower.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(idx) = lower.parse::<u16>() {
            if idx <= 255 {
                return Some(format!("38;5;{idx}"));
            }
        }
    }

    if let Some(code) = named_color(&lower) {
        return Some(code.to_string());
    }

    None
}

fn named_color(name: &str) -> Option<&'static str> {
    match name {
        "black" => Some("30"),
        "red" => Some("31"),
        "green" => Some("32"),
        "yellow" => Some("33"),
        "blue" => Some("34"),
        "magenta" | "purple" => Some("35"),
        "cyan" => Some("36"),
        "white" => Some("37"),
        "default" => Some("39"),
        "bright_black" | "light_black" | "grey" | "gray" => Some("90"),
        "bright_red" | "light_red" => Some("91"),
        "bright_green" | "light_green" => Some("92"),
        "bright_yellow" | "light_yellow" => Some("93"),
        "bright_blue" | "light_blue" => Some("94"),
        "bright_magenta" | "light_magenta" | "bright_purple" | "light_purple" => Some("95"),
        "bright_cyan" | "light_cyan" => Some("96"),
        "bright_white" | "light_white" => Some("97"),
        "bold" => Some("1"),
        "dim" => Some("2"),
        "italic" => Some("3"),
        "underline" => Some("4"),
        "blink" => Some("5"),
        "inverse" => Some("7"),
        "hidden" => Some("8"),
        "strike" => Some("9"),
        _ => None,
    }
}

fn parse_rgb_values(spec: &str) -> Option<String> {
    let parts: Vec<&str> = spec
        .split(|c: char| c == ';' || c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    let r = parts[0].parse::<u16>().ok()?;
    let g = parts[1].parse::<u16>().ok()?;
    let b = parts[2].parse::<u16>().ok()?;
    if r > 255 || g > 255 || b > 255 {
        return None;
    }
    Some(format!("38;2;{r};{g};{b}"))
}

fn parse_palette_index(spec: &str) -> Option<u8> {
    let idx = spec.parse::<u16>().ok()?;
    if idx <= 255 {
        Some(idx as u8)
    } else {
        None
    }
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim();
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some((r, g, b));
    }
    if hex.len() == 3 {
        let expanded = hex
            .chars()
            .flat_map(|c| std::iter::repeat(c).take(2))
            .collect::<String>();
        return parse_hex_color(&expanded);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_named_colors() {
        assert_eq!(parse_color_spec("blue"), Some("34".to_string()));
        assert_eq!(parse_color_spec("bright_red"), Some("91".to_string()));
    }

    #[test]
    fn parse_hex_formats() {
        assert_eq!(
            parse_color_spec("#ff0000"),
            Some("38;2;255;0;0".to_string())
        );
        assert_eq!(parse_color_spec("#f00"), Some("38;2;255;0;0".to_string()));
    }

    #[test]
    fn parse_rgb_formats() {
        assert_eq!(
            parse_color_spec("rgb:255;128;0"),
            Some("38;2;255;128;0".to_string())
        );
        assert_eq!(
            parse_color_spec("rgb(255,128,0)"),
            Some("38;2;255;128;0".to_string())
        );
    }

    #[test]
    fn parse_palette() {
        assert_eq!(parse_color_spec("256:160"), Some("38;5;160".to_string()));
        assert_eq!(parse_color_spec("196"), Some("38;5;196".to_string()));
    }
}
