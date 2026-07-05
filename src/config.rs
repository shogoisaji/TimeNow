use crate::{
    clock::{Color, DateDisplay, Style, Theme},
    digits::Font,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub style: Style,
    pub theme: Theme,
    pub date_display: DateDisplay,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            style: Style::DEFAULT,
            theme: Theme {
                fg: Color::Cyan,
                bg: Color::Black,
            },
            date_display: DateDisplay::default(),
        }
    }
}

pub fn parse_config(text: &str, default: AppConfig) -> AppConfig {
    let mut config = default;
    if let Some(font) = json_string(text, "font").as_deref().and_then(font_from_id) {
        config.style.font = font;
    }
    if let Some(chars) = json_string(text, "symbol")
        .as_deref()
        .and_then(charset_from_id)
    {
        config.style.chars = chars;
    }
    if let Some(fg) = json_string(text, "foreground")
        .as_deref()
        .and_then(color_from_id)
    {
        config.theme.fg = fg;
    }
    if let Some(bg) = json_string(text, "background")
        .as_deref()
        .and_then(color_from_id)
    {
        config.theme.bg = bg;
    }
    if let Some(date_display) = json_string(text, "date").as_deref().and_then(date_from_id) {
        config.date_display = date_display;
    }
    config
}

pub fn format_config(config: AppConfig) -> String {
    format!(
        "{{\n  \"font\": \"{}\",\n  \"symbol\": \"{}\",\n  \"foreground\": \"{}\",\n  \"background\": \"{}\",\n  \"date\": \"{}\"\n}}\n",
        font_id(config.style.font),
        charset_id(config.style.chars),
        color_id(config.theme.fg),
        color_id(config.theme.bg),
        date_id(config.date_display),
    )
}

fn json_string(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let after_key = text.split_once(&needle)?.1;
    let after_colon = after_key.split_once(':')?.1.trim_start();
    let mut chars = after_colon.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut value = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            value.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(value);
        } else {
            value.push(ch);
        }
    }
    None
}

fn font_id(font: Font) -> &'static str {
    match font {
        Font::Neo => "Neo",
        Font::Block => "Block",
        Font::Outline => "Outline",
        Font::Segment => "Segment",
        Font::Dot | Font::Slant => "Neo",
    }
}

fn font_from_id(id: &str) -> Option<Font> {
    match id {
        "Neo" => Some(Font::Neo),
        "Block" => Some(Font::Block),
        "Outline" => Some(Font::Outline),
        "Segment" => Some(Font::Segment),
        _ => None,
    }
}

fn charset_id(chars: crate::clock::CharSet) -> &'static str {
    use crate::clock::CharSet;
    match chars {
        CharSet::Fill { on: '#', off: ' ' } => "Hash",
        CharSet::Fill {
            on: '█', off: ' '
        } => "Block",
        CharSet::Fill {
            on: '▓', off: '░'
        } => "Shade",
        CharSet::Fill {
            on: '▒', off: ' '
        } => "Shade2",
        CharSet::Fill {
            on: '█', off: '▄'
        } => "FullLower",
        CharSet::Fill {
            on: '█', off: '▀'
        } => "FullUpper",
        CharSet::Fill {
            on: '█', off: '▒'
        } => "FullMid",
        CharSet::Fill {
            on: '█', off: '░'
        } => "FullLight",
        CharSet::Dots => "Dots",
        _ => "Block",
    }
}

fn charset_from_id(id: &str) -> Option<crate::clock::CharSet> {
    use crate::clock::CharSet;
    match id {
        "Hash" => Some(CharSet::Fill { on: '#', off: ' ' }),
        "Block" => Some(CharSet::Fill {
            on: '█', off: ' '
        }),
        "Shade" => Some(CharSet::Fill {
            on: '▓', off: '░'
        }),
        "Shade2" => Some(CharSet::Fill {
            on: '▒', off: ' '
        }),
        "FullLower" => Some(CharSet::Fill {
            on: '█', off: '▄'
        }),
        "FullUpper" => Some(CharSet::Fill {
            on: '█', off: '▀'
        }),
        "FullMid" => Some(CharSet::Fill {
            on: '█', off: '▒'
        }),
        "FullLight" => Some(CharSet::Fill {
            on: '█', off: '░'
        }),
        "Dots" => Some(CharSet::Dots),
        // 廃止された旧IDはデフォルトへフォールバック
        "StarDot" | "PlusMinus" | "Half" | "Square" | "Outline" => None,
        _ => None,
    }
}

fn color_id(color: Color) -> &'static str {
    match color {
        Color::Default => "Default",
        Color::Black => "Black",
        Color::Red => "Red",
        Color::Green => "Green",
        Color::Yellow => "Yellow",
        Color::Blue => "Blue",
        Color::Magenta => "Magenta",
        Color::Cyan => "Cyan",
        Color::White => "White",
    }
}

fn color_from_id(id: &str) -> Option<Color> {
    match id {
        "Default" => Some(Color::Default),
        "Black" => Some(Color::Black),
        "Red" => Some(Color::Red),
        "Green" => Some(Color::Green),
        "Yellow" => Some(Color::Yellow),
        "Blue" => Some(Color::Blue),
        "Magenta" => Some(Color::Magenta),
        "Cyan" => Some(Color::Cyan),
        "White" => Some(Color::White),
        _ => None,
    }
}

fn date_id(display: DateDisplay) -> &'static str {
    match display {
        DateDisplay::None => "None",
        DateDisplay::Numeric => "Numeric",
    }
}

fn date_from_id(id: &str) -> Option<DateDisplay> {
    match id {
        "None" => Some(DateDisplay::None),
        "Numeric" => Some(DateDisplay::Numeric),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::CharSet;

    fn sample_config() -> AppConfig {
        AppConfig {
            style: Style {
                font: Font::Segment,
                chars: CharSet::Dots,
            },
            theme: Theme {
                fg: Color::Yellow,
                bg: Color::Blue,
            },
            date_display: DateDisplay::None,
        }
    }

    #[test]
    fn default_config_matches_runtime_defaults() {
        assert_eq!(
            AppConfig::default(),
            AppConfig {
                style: Style::DEFAULT,
                theme: Theme {
                    fg: Color::Cyan,
                    bg: Color::Black,
                },
                date_display: DateDisplay::Numeric,
            }
        );
    }

    #[test]
    fn config_round_trips_style_theme_and_date() {
        let config = sample_config();

        let text = format_config(config);

        assert_eq!(parse_config(&text, AppConfig::default()), config);
    }

    #[test]
    fn format_config_uses_stable_json_shape() {
        assert_eq!(
            format_config(sample_config()),
            "{\n  \"font\": \"Segment\",\n  \"symbol\": \"Dots\",\n  \"foreground\": \"Yellow\",\n  \"background\": \"Blue\",\n  \"date\": \"None\"\n}\n"
        );
    }

    #[test]
    fn parse_config_keeps_defaults_for_empty_config() {
        let default = sample_config();

        assert_eq!(parse_config("{}", default), default);
    }

    #[test]
    fn parse_config_applies_partial_overrides() {
        let default = sample_config();
        let parsed = parse_config(
            "{\n  \"font\": \"Outline\",\n  \"foreground\": \"Magenta\"\n}",
            default,
        );

        assert_eq!(parsed.style.font, Font::Outline);
        assert_eq!(parsed.style.chars, default.style.chars);
        assert_eq!(parsed.theme.fg, Color::Magenta);
        assert_eq!(parsed.theme.bg, default.theme.bg);
        assert_eq!(parsed.date_display, default.date_display);
    }

    #[test]
    fn parse_config_ignores_unknown_values() {
        let default = sample_config();
        let parsed = parse_config(
            "{\n  \"font\": \"FutureFont\",\n  \"symbol\": \"Emoji\",\n  \"foreground\": \"Orange\",\n  \"background\": \"Transparent\",\n  \"date\": \"Weekday\"\n}",
            default,
        );

        assert_eq!(parsed, default);
    }

    #[test]
    fn parse_config_accepts_all_supported_symbols() {
        let cases: [(&str, CharSet); 9] = [
            ("Hash", CharSet::Fill { on: '#', off: ' ' }),
            (
                "Block",
                CharSet::Fill {
                    on: '█', off: ' '
                },
            ),
            (
                "Shade",
                CharSet::Fill {
                    on: '▓', off: '░'
                },
            ),
            (
                "Shade2",
                CharSet::Fill {
                    on: '▒', off: ' '
                },
            ),
            (
                "FullLower",
                CharSet::Fill {
                    on: '█', off: '▄'
                },
            ),
            (
                "FullUpper",
                CharSet::Fill {
                    on: '█', off: '▀'
                },
            ),
            (
                "FullMid",
                CharSet::Fill {
                    on: '█', off: '▒'
                },
            ),
            (
                "FullLight",
                CharSet::Fill {
                    on: '█', off: '░'
                },
            ),
            ("Dots", CharSet::Dots),
        ];

        for (id, chars) in cases {
            let text = format!("{{\"symbol\": \"{id}\"}}");

            assert_eq!(parse_config(&text, AppConfig::default()).style.chars, chars);
        }
    }

    #[test]
    fn parse_config_retired_symbol_ids_fall_back_to_default() {
        // 廃止された旧IDはデフォルトへフォールバック
        let default = AppConfig::default();
        for id in ["StarDot", "PlusMinus", "Half", "Square", "Outline"] {
            assert_eq!(
                parse_config(&format!("{{\"symbol\": \"{id}\"}}"), default)
                    .style
                    .chars,
                default.style.chars
            );
        }
    }

    #[test]
    fn parse_config_falls_back_to_default_for_unknown_date_value() {
        // "MonthDay" は廃止された値。未知の値はデフォルト(Numeric)にフォールバックする。
        assert_eq!(
            parse_config("{\"date\": \"MonthDay\"}", AppConfig::default()).date_display,
            DateDisplay::Numeric
        );
    }

    #[test]
    fn format_config_normalizes_retired_font_and_symbol() {
        let config = AppConfig {
            style: Style {
                font: Font::Dot,
                chars: CharSet::Fill { on: 'x', off: 'o' },
            },
            theme: Theme::default(),
            date_display: DateDisplay::Numeric,
        };

        let text = format_config(config);

        assert!(text.contains("\"font\": \"Neo\""));
        assert!(text.contains("\"symbol\": \"Block\""));
        assert!(text.contains("\"date\": \"Numeric\""));
    }
}
