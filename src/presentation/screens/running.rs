use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::presentation::{app::App, theme};

pub fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    if !theme::supports_minimum_size(area) {
        frame.render_widget(
            Paragraph::new("Увеличьте окно терминала до 80×24")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }

    let panel = theme::centered(area, area.width * 75 / 100, area.height * 55 / 100);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(panel);

    frame.render_widget(
        Paragraph::new("ED Planet Search")
            .alignment(Alignment::Center)
            .style(theme::Theme::default().accent)
            .block(Block::default().title(" Поиск планет ").borders(Borders::ALL)),
        sections[0],
    );

    let system = match app.resolved_system.as_deref() {
        Some(name) => Line::from(vec![
            Span::styled("✓ ", theme::Theme::default().success),
            Span::raw(format!("Центральная система найдена: {name}")),
        ]),
        None => Line::from(vec![
            Span::styled("● ", theme::Theme::default().accent),
            Span::raw("Поиск центральной системы…"),
        ]),
    };
    let pages = if app.current_page > 0 {
        Line::from(vec![
            Span::styled("✓ ", theme::Theme::default().success),
            Span::raw(format!("Загружена страница Spansh: {}", app.current_page)),
        ])
    } else {
        Line::from(vec![
            Span::styled("○ ", theme::Theme::default().muted),
            Span::raw("Ожидание первой страницы Spansh…"),
        ])
    };
    let content = vec![
        system,
        pages,
        Line::from(format!(
            "Подходящих планет: {} из {}",
            app.found_count, app.config.criteria.count
        )),
        Line::from(
            app.progress_message
                .as_deref()
                .unwrap_or("Подготовка запроса…"),
        ),
    ];
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" Выполнение ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ratatui::style::Color::DarkGray)),
            ),
        sections[1],
    );
    frame.render_widget(
        Paragraph::new("Esc — отменить поиск")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP)),
        sections[2],
    );
}
