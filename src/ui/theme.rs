use ratatui::style::Color;

#[derive(Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub base: Color,
    pub surface0: Color,
    pub surface1: Color,
    pub surface2: Color,
    pub overlay0: Color,
    pub overlay1: Color,
    pub overlay2: Color,
    pub subtext0: Color,
    pub subtext1: Color,
    pub text: Color,
    pub accent_blue: Color,
    pub accent_teal: Color,
    pub accent_yellow: Color,
    pub accent_green: Color,
    pub accent_red: Color,
    pub accent_orange: Color,
    pub accent_purple: Color,
    pub bg_dark: Color,
    pub bg_panel: Color,
    pub border: Color,
}

pub fn catppuccin_mocha() -> Theme {
    Theme {
        base: Color::Rgb(30, 30, 46),
        surface0: Color::Rgb(49, 50, 68),
        surface1: Color::Rgb(69, 71, 90),
        surface2: Color::Rgb(88, 91, 112),
        overlay0: Color::Rgb(108, 112, 134),
        // 4.75:1 on bg_panel — minimum for secondary text (measured).
        overlay1: Color::Rgb(132, 137, 162),
        overlay2: Color::Rgb(147, 153, 178),
        subtext0: Color::Rgb(166, 173, 200),
        subtext1: Color::Rgb(186, 194, 222),
        text: Color::Rgb(205, 214, 244),
        accent_blue: Color::Rgb(137, 180, 250),
        accent_teal: Color::Rgb(148, 226, 213),
        accent_yellow: Color::Rgb(249, 226, 175),
        accent_green: Color::Rgb(166, 227, 161),
        accent_red: Color::Rgb(243, 139, 168),
        accent_orange: Color::Rgb(250, 179, 135),
        accent_purple: Color::Rgb(203, 166, 247),
        bg_dark: Color::Rgb(24, 24, 37),
        bg_panel: Color::Rgb(30, 30, 46),
        border: Color::Rgb(69, 71, 90),
    }
}

pub fn high_contrast() -> Theme {
    Theme {
        base: Color::Rgb(0, 0, 0),
        surface0: Color::Rgb(18, 18, 18),
        surface1: Color::Rgb(36, 36, 36),
        surface2: Color::Rgb(58, 58, 58),
        overlay0: Color::Rgb(180, 180, 180),
        overlay1: Color::Rgb(205, 205, 205),
        overlay2: Color::Rgb(225, 225, 225),
        subtext0: Color::Rgb(218, 218, 218),
        subtext1: Color::Rgb(238, 238, 238),
        text: Color::Rgb(255, 255, 255),
        accent_blue: Color::Rgb(96, 200, 255),
        accent_teal: Color::Rgb(95, 255, 230),
        accent_yellow: Color::Rgb(255, 238, 90),
        accent_green: Color::Rgb(118, 255, 128),
        accent_red: Color::Rgb(255, 112, 112),
        accent_orange: Color::Rgb(255, 181, 96),
        accent_purple: Color::Rgb(205, 170, 255),
        bg_dark: Color::Rgb(0, 0, 0),
        bg_panel: Color::Rgb(0, 0, 0),
        border: Color::Rgb(150, 150, 150),
    }
}

pub fn colorblind_safe() -> Theme {
    Theme {
        base: Color::Rgb(18, 18, 18),
        surface0: Color::Rgb(32, 32, 32),
        surface1: Color::Rgb(48, 48, 48),
        surface2: Color::Rgb(70, 70, 70),
        overlay0: Color::Rgb(168, 168, 168),
        overlay1: Color::Rgb(196, 196, 196),
        overlay2: Color::Rgb(220, 220, 220),
        subtext0: Color::Rgb(210, 210, 210),
        subtext1: Color::Rgb(232, 232, 232),
        text: Color::Rgb(250, 250, 250),
        accent_blue: Color::Rgb(86, 180, 233),
        accent_teal: Color::Rgb(0, 204, 178),
        accent_yellow: Color::Rgb(240, 228, 66),
        accent_green: Color::Rgb(0, 210, 140),
        accent_red: Color::Rgb(255, 120, 92),
        accent_orange: Color::Rgb(230, 159, 0),
        accent_purple: Color::Rgb(204, 121, 167),
        bg_dark: Color::Rgb(12, 12, 12),
        bg_panel: Color::Rgb(18, 18, 18),
        border: Color::Rgb(145, 145, 145),
    }
}

use std::sync::OnceLock;
static THEME: OnceLock<Theme> = OnceLock::new();

pub fn configure(name: Option<&str>) {
    let theme = match name
        .unwrap_or("default")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "high-contrast" | "high_contrast" | "contrast" | "accessible" => high_contrast(),
        "colorblind" | "colorblind-safe" | "colorblind_safe" => colorblind_safe(),
        _ => catppuccin_mocha(),
    };
    let _ = THEME.set(theme);
}

pub fn get() -> &'static Theme {
    THEME.get_or_init(catppuccin_mocha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessible_themes_meet_contrast_targets() {
        for theme in [high_contrast(), colorblind_safe()] {
            // Accessible themes keep even dim text above AA.
            assert_body_contrast(
                &theme,
                4.5,
                &[
                    theme.text,
                    theme.subtext0,
                    theme.subtext1,
                    theme.overlay0,
                    theme.overlay1,
                    theme.overlay2,
                    theme.accent_blue,
                    theme.accent_teal,
                    theme.accent_yellow,
                    theme.accent_green,
                    theme.accent_red,
                    theme.accent_orange,
                    theme.accent_purple,
                ],
            );
            assert!(contrast_ratio(theme.border, theme.bg_panel) >= 3.0);
        }
    }

    #[test]
    fn default_theme_body_text_meets_aa() {
        // overlay0 (~3.4:1) stays decorative-only in the default theme;
        // everything that carries information must clear AA.
        let theme = catppuccin_mocha();
        assert_body_contrast(
            &theme,
            4.5,
            &[
                theme.text,
                theme.subtext0,
                theme.subtext1,
                theme.overlay1,
                theme.overlay2,
                theme.accent_blue,
                theme.accent_teal,
                theme.accent_yellow,
                theme.accent_green,
                theme.accent_red,
                theme.accent_orange,
                theme.accent_purple,
            ],
        );
    }

    fn assert_body_contrast(theme: &Theme, min_ratio: f64, colors: &[Color]) {
        for color in colors {
            assert!(
                contrast_ratio(*color, theme.bg_panel) >= min_ratio,
                "contrast {:?} on {:?} is {:.2}",
                color,
                theme.bg_panel,
                contrast_ratio(*color, theme.bg_panel)
            );
        }
    }

    fn contrast_ratio(fg: Color, bg: Color) -> f64 {
        let fg_l = relative_luminance(fg);
        let bg_l = relative_luminance(bg);
        let lighter = fg_l.max(bg_l);
        let darker = fg_l.min(bg_l);
        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(color: Color) -> f64 {
        let Color::Rgb(r, g, b) = color else {
            panic!("theme tests require RGB colors");
        };
        0.2126 * srgb_channel(r) + 0.7152 * srgb_channel(g) + 0.0722 * srgb_channel(b)
    }

    fn srgb_channel(value: u8) -> f64 {
        let value = value as f64 / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
}
