use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};

pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 24;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub accent: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub muted: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            success: Style::default().fg(Color::Green),
            warning: Style::default().fg(Color::Yellow),
            error: Style::default().fg(Color::Red),
            muted: Style::default().fg(Color::DarkGray),
        }
    }
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(1);
    let height = height.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

pub fn supports_minimum_size(area: Rect) -> bool {
    area.width >= MIN_WIDTH && area.height >= MIN_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_panel_stays_inside_area_and_is_balanced() {
        let area = Rect::new(10, 4, 100, 40);
        let panel = centered(area, 60, 20);

        assert_eq!(panel, Rect::new(30, 14, 60, 20));
    }

    #[test]
    fn centered_panel_clamps_to_small_area() {
        let area = Rect::new(0, 0, 60, 18);
        let panel = centered(area, 72, 25);

        assert_eq!(panel, Rect::new(1, 1, 58, 16));
    }

    #[test]
    fn minimum_size_is_eighty_by_twenty_four() {
        assert!(supports_minimum_size(Rect::new(0, 0, 80, 24)));
        assert!(supports_minimum_size(Rect::new(0, 0, 120, 40)));
        assert!(!supports_minimum_size(Rect::new(0, 0, 79, 24)));
        assert!(!supports_minimum_size(Rect::new(0, 0, 80, 23)));
    }
}
