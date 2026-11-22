use ratatui::{
    layout::Alignment,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::presentation::{app::App, text::ui, theme};

pub fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    if !theme::supports_minimum_size(area) {
        frame.render_widget(
            Paragraph::new(ui::MIN_TERMINAL_SIZE)
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }
    let panel = theme::centered(area, area.width * 80 / 100, area.height * 55 / 100);
    let message = app.error_message.as_deref().unwrap_or(ui::ERROR_FALLBACK);

    frame.render_widget(
        Paragraph::new(format!(
            "{}\n\n{message}\n\n{}",
            ui::ERROR_HEADING,
            ui::ERROR_HINTS
        ))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(ui::ERROR_TITLE)
                .borders(Borders::ALL)
                .border_style(theme::Theme::default().error),
        ),
        panel,
    );
}
