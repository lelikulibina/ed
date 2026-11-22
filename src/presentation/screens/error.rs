use ratatui::{
    layout::Alignment,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::presentation::{app::App, theme};

pub fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    let panel = theme::centered(area, area.width * 80 / 100, area.height * 55 / 100);
    let message = app
        .error_message
        .as_deref()
        .unwrap_or("Не удалось выполнить поиск. Попробуйте ещё раз.");

    frame.render_widget(
        Paragraph::new(format!(
            "Поиск не выполнен\n\n{message}\n\nr — повторить поиск\ne / Enter / Esc — изменить параметры\nCtrl+Q — выход"
        ))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Ошибка поиска ")
                .borders(Borders::ALL)
                .border_style(theme::Theme::default().error),
        ),
        panel,
    );
}
