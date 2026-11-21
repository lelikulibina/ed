use ratatui::layout::Rect;

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
    }
}
