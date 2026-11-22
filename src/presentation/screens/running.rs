use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Span},
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
        Paragraph::new(ui::APP_TITLE)
            .alignment(Alignment::Center)
            .style(theme::Theme::default().accent)
            .block(
                Block::default()
                    .title(ui::RUNNING_TITLE)
                    .borders(Borders::ALL),
            ),
        sections[0],
    );

    let system = match app.resolved_system.as_deref() {
        Some(name) => Line::from(vec![
            Span::styled("✓ ", theme::Theme::default().success),
            Span::raw(format!("{}: {name}", ui::RUNNING_SYSTEM_FOUND_PREFIX)),
        ]),
        None => Line::from(vec![
            Span::styled("● ", theme::Theme::default().accent),
            Span::raw(ui::RUNNING_SYSTEM_PENDING),
        ]),
    };
    let replacement = app.resolved_system.as_deref().filter(|resolved| {
        !resolved.trim().eq_ignore_ascii_case(app.config.system.trim())
    });
    let pages = if app.current_page > 0 {
        Line::from(vec![
            Span::styled("✓ ", theme::Theme::default().success),
            Span::raw(format!(
                "{}: {}",
                ui::RUNNING_PAGE_LOADED_PREFIX,
                app.current_page
            )),
        ])
    } else {
        Line::from(vec![
            Span::styled("○ ", theme::Theme::default().muted),
            Span::raw(ui::RUNNING_PAGE_PENDING),
        ])
    };
    let mut content = vec![system];
    if let Some(resolved) = replacement {
        content.push(Line::from(Span::styled(
            format!(
                "{}: «{}» → «{}»",
                ui::SYSTEM_REPLACED_WARNING,
                app.config.system,
                resolved
            ),
            theme::Theme::default().warning,
        )));
    }
    content.extend([

        pages,
        Line::from(format!(
            "{}: {} {} {}",
            ui::RUNNING_FOUND_COUNT,
            app.found_count,
            ui::RUNNING_COUNT_SEPARATOR,
            app.config.criteria.count
        )),
        Line::from(
            app.progress_message
                .as_deref()
                .unwrap_or(ui::RUNNING_PREPARING),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(content).wrap(Wrap { trim: true }).block(
            Block::default()
                .title(ui::RUNNING_PANEL_TITLE)
                .borders(Borders::ALL)
                .border_style(theme::Theme::default().muted),
        ),
        sections[1],
    );
    frame.render_widget(
        Paragraph::new(ui::RUNNING_CANCEL_HINT)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP)),
        sections[2],
    );
}
