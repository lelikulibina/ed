use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    Medium,
    Compact,
    TooSmall,
}

/// Selects a layout that leaves enough room for the persistent controls.
pub fn layout_mode(area: Rect) -> LayoutMode {
    match (area.width, area.height) {
        (116.., 32..) => LayoutMode::Wide,
        (96.., 26..) => LayoutMode::Medium,
        (64.., 18..) => LayoutMode::Compact,
        _ => LayoutMode::TooSmall,
    }
}

#[cfg(test)]
mod tests {
    use super::{LayoutMode, layout_mode};
    use ratatui::layout::Rect;

    #[test]
    fn chooses_layout_for_reference_terminal_sizes() {
        assert_eq!(layout_mode(Rect::new(0, 0, 120, 40)), LayoutMode::Wide);
        assert_eq!(layout_mode(Rect::new(0, 0, 100, 30)), LayoutMode::Medium);
        assert_eq!(layout_mode(Rect::new(0, 0, 80, 24)), LayoutMode::Compact);
    }

    #[test]
    fn recognizes_too_small_terminal_areas() {
        assert_eq!(layout_mode(Rect::new(0, 0, 40, 12)), LayoutMode::TooSmall);
        assert_eq!(layout_mode(Rect::new(0, 0, 0, 0)), LayoutMode::TooSmall);
        assert_eq!(layout_mode(Rect::new(0, 0, 120, 17)), LayoutMode::TooSmall);
    }
}
