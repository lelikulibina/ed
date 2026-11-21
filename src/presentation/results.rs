use std::{cmp::Ordering, collections::HashSet};

use crate::domain::body::Body;

use super::layout::LayoutMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Surface,
    Atmosphere,
    Temperature,
    Pressure,
    Gravity,
    Distance,
}

impl SortColumn {
    pub const ALL: [Self; 7] = [
        Self::Name,
        Self::Surface,
        Self::Atmosphere,
        Self::Temperature,
        Self::Pressure,
        Self::Gravity,
        Self::Distance,
    ];

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|column| *column == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Планета",
            Self::Surface => "Поверхность",
            Self::Atmosphere => "Атмосфера",
            Self::Temperature => "Темп., K",
            Self::Pressure => "Давл., atm",
            Self::Gravity => "Грав., g",
            Self::Distance => "Расст., св. л.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(&mut self) {
        *self = match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        };
    }

    pub fn indicator(self) -> &'static str {
        match self {
            Self::Ascending => " ↑",
            Self::Descending => " ↓",
        }
    }
}

#[derive(Debug)]
pub struct ResultsState {
    pub selected: usize,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
    pub marked: HashSet<String>,
    pub details_open: bool,
    pub detail_scroll: u16,
    pub help_open: bool,
}

impl ResultsState {
    pub fn new(_bodies: &[Body]) -> Self {
        Self {
            selected: 0,
            sort_column: SortColumn::Distance,
            sort_direction: SortDirection::Ascending,
            marked: HashSet::new(),
            details_open: false,
            detail_scroll: 0,
            help_open: false,
        }
    }

    pub fn select_next(&mut self, len: usize) {
        if len > 0 {
            self.selected = self.selected.saturating_add(1).min(len - 1);
        } else {
            self.selected = 0;
        }
    }

    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    pub fn select_last(&mut self, len: usize) {
        self.selected = len.saturating_sub(1);
    }

    pub fn toggle_mark(&mut self, body: &Body) {
        let identity = body_identity(body);
        if !self.marked.remove(&identity) {
            self.marked.insert(identity);
        }
    }

    pub fn is_marked(&self, body: &Body) -> bool {
        self.marked.contains(&body_identity(body))
    }
}

pub fn body_identity(body: &Body) -> String {
    if let Some(id64) = body.id64.as_deref() {
        return format!("body:{id64}");
    }
    let system = body
        .system_id64
        .as_deref()
        .or(body.system_name.as_deref())
        .unwrap_or("unknown-system");
    format!("system:{system}/body:{}", body.name)
}

pub fn visible_columns(mode: LayoutMode) -> Vec<SortColumn> {
    match mode {
        LayoutMode::Wide => SortColumn::ALL.to_vec(),
        LayoutMode::Medium => vec![
            SortColumn::Name,
            SortColumn::Surface,
            SortColumn::Atmosphere,
            SortColumn::Temperature,
            SortColumn::Distance,
        ],
        LayoutMode::Compact | LayoutMode::TooSmall => vec![
            SortColumn::Name,
            SortColumn::Temperature,
            SortColumn::Distance,
        ],
    }
}

pub fn format_optional(value: Option<f64>, precision: usize, suffix: &str) -> String {
    match value.filter(|value| value.is_finite()) {
        Some(value) => format!("{value:.precision$}{suffix}"),
        None => "—".to_owned(),
    }
}

fn compare_optional<T: Ord>(
    left: Option<T>,
    right: Option<T>,
    direction: SortDirection,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => {
            let order = left.cmp(&right);
            match direction {
                SortDirection::Ascending => order,
                SortDirection::Descending => order.reverse(),
            }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_number(
    left: Option<f64>,
    right: Option<f64>,
    direction: SortDirection,
) -> Ordering {
    match (
        left.filter(|value| value.is_finite()),
        right.filter(|value| value.is_finite()),
    ) {
        (Some(left), Some(right)) => {
            let order = left.total_cmp(&right);
            match direction {
                SortDirection::Ascending => order,
                SortDirection::Descending => order.reverse(),
            }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_text(left: Option<&str>, right: Option<&str>, direction: SortDirection) -> Ordering {
    compare_optional(
        left.map(str::to_lowercase),
        right.map(str::to_lowercase),
        direction,
    )
}

pub fn sort_bodies(bodies: &mut [Body], column: SortColumn, direction: SortDirection) {
    bodies.sort_by(|left, right| match column {
        SortColumn::Name => compare_text(Some(&left.name), Some(&right.name), direction),
        SortColumn::Surface => {
            compare_text(left.subtype.as_deref(), right.subtype.as_deref(), direction)
        }
        SortColumn::Atmosphere => compare_text(
            left.atmosphere.as_deref(),
            right.atmosphere.as_deref(),
            direction,
        ),
        SortColumn::Temperature => compare_optional_number(
            left.surface_temperature,
            right.surface_temperature,
            direction,
        ),
        SortColumn::Pressure => {
            compare_optional_number(left.surface_pressure, right.surface_pressure, direction)
        }
        SortColumn::Gravity => compare_optional_number(left.gravity, right.gravity, direction),
        SortColumn::Distance => {
            compare_optional_number(left.distance_ly, right.distance_ly, direction)
        }
    });
}

pub fn results_help_lines() -> Vec<ratatui::text::Line<'static>> {
    use ratatui::text::Line;

    vec![
        Line::from("УПРАВЛЕНИЕ РЕЗУЛЬТАТАМИ"),
        Line::from(""),
        Line::from("↑ / ↓       Выбрать предыдущую / следующую планету"),
        Line::from("Home / End  Перейти к началу / концу таблицы"),
        Line::from("s           Следующий столбец сортировки"),
        Line::from("r           Изменить направление сортировки"),
        Line::from("Space       Отметить / снять отметку с планеты"),
        Line::from("Enter       Открыть подробности выбранной планеты"),
        Line::from("Esc         Закрыть подробности или вернуться к настройкам"),
        Line::from("?           Закрыть / открыть эту справку"),
        Line::from("Ctrl+Q      Завершить приложение"),
    ]
}

pub fn cell_value(body: &Body, column: SortColumn) -> String {
    match column {
        SortColumn::Name => body.name.clone(),
        SortColumn::Surface => body.subtype.as_deref().unwrap_or("—").to_owned(),
        SortColumn::Atmosphere => body.atmosphere.as_deref().unwrap_or("—").to_owned(),
        SortColumn::Temperature => format_optional(body.surface_temperature, 1, ""),
        SortColumn::Pressure => format_optional(body.surface_pressure, 3, ""),
        SortColumn::Gravity => format_optional(body.gravity, 3, ""),
        SortColumn::Distance => format_optional(body.distance_ly, 1, ""),
    }
}

pub fn body_summary_lines(body: &Body) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::text::Line;

    vec![
        Line::from(body.name.clone()),
        Line::from(format!(
            "Система: {}",
            body.system_name.as_deref().unwrap_or("нет данных")
        )),
        Line::from(format!(
            "Поверхность: {}",
            body.subtype.as_deref().unwrap_or("—")
        )),
        Line::from(format!(
            "Атмосфера: {}",
            body.atmosphere.as_deref().unwrap_or("—")
        )),
        Line::from(format!(
            "Температура: {}",
            format_optional(body.surface_temperature, 1, " K")
        )),
        Line::from(format!(
            "Расстояние: {}",
            format_optional(body.distance_ly, 1, " св. л.")
        )),
        Line::from(format!(
            "Картографирована: {}",
            optional_bool(body.is_mapped)
        )),
        Line::from(format!(
            "Биологические сигналы: {}",
            optional_count(body.biological_signal_count)
        )),
    ]
}

pub fn body_detail_lines(body: &Body) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::text::Line;

    let mut lines = vec![
        Line::from("ИДЕНТИФИКАЦИЯ"),
        Line::from(format!("Планета: {}", body.name)),
        Line::from(format!(
            "Система: {}",
            body.system_name.as_deref().unwrap_or("нет данных")
        )),
        Line::from(format!(
            "ID тела: {}",
            body.id64.as_deref().unwrap_or("нет данных")
        )),
        Line::from(format!(
            "Тип поверхности: {}",
            body.subtype.as_deref().unwrap_or("нет данных")
        )),
        Line::from(""),
        Line::from("ФИЗИКА"),
        Line::from(format!(
            "Атмосфера: {}",
            body.atmosphere.as_deref().unwrap_or("нет данных")
        )),
        Line::from(format!(
            "Температура: {}",
            format_optional(body.surface_temperature, 2, " K")
        )),
        Line::from(format!(
            "Давление: {}",
            format_optional(body.surface_pressure, 4, " atm")
        )),
        Line::from(format!(
            "Гравитация: {}",
            format_optional(body.gravity, 3, " g")
        )),
        Line::from(format!(
            "Расстояние: {}",
            format_optional(body.distance_ly, 2, " св. л.")
        )),
        Line::from(format!(
            "Можно приземлиться: {}",
            optional_bool(body.is_landable)
        )),
        Line::from(""),
        Line::from("ИССЛЕДОВАНИЕ"),
        Line::from(format!(
            "Картографирована: {}",
            optional_bool(body.is_mapped)
        )),
        Line::from(format!(
            "Обновление записи: {}",
            body.last_updated_at.as_deref().unwrap_or("нет данных")
        )),
        Line::from(format!(
            "Биологические сигналы Spansh: {}",
            optional_count(body.biological_signal_count)
        )),
        Line::from(format!(
            "Геологические сигналы Spansh: {}",
            optional_count(body.geological_signal_count)
        )),
        Line::from(format!(
            "Обновление сигналов: {}",
            body.signals_updated_at.as_deref().unwrap_or("нет данных")
        )),
        Line::from(""),
        Line::from("ЭКЗОБИОЛОГИЯ"),
        Line::from(format!(
            "Биологические группы: {}",
            body.biological_genera
                .as_ref()
                .filter(|genera| !genera.is_empty())
                .map_or_else(
                    || "не указаны".to_owned(),
                    |genera| genera
                        .iter()
                        .map(|genus| crate::presentation::output::russian_group_name(genus))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
        )),
    ];

    match crate::presentation::output::exobiology_summary(body) {
        Ok(Some(summary)) => lines.extend(summary.lines().map(|line| Line::from(line.to_owned()))),
        Ok(None) => lines.push(Line::from("Подходящие виды в справочнике не найдены.")),
        Err(error) => lines.push(Line::from(format!("Ошибка расчёта экзобиологии: {error}"))),
    }

    lines.extend([
        Line::from(""),
        Line::from("ГЕОЛОГИЯ"),
        Line::from(format!(
            "Вулканизм: {}",
            body.volcanism_type.as_deref().unwrap_or("нет данных")
        )),
    ]);
    lines
}

fn optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "да",
        Some(false) => "нет",
        None => "нет данных",
    }
}

fn optional_count(value: Option<u32>) -> String {
    value.map_or_else(|| "нет данных".to_owned(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        LayoutMode, ResultsState, SortColumn, SortDirection, body_detail_lines, body_identity,
        body_summary_lines, format_optional, results_help_lines, sort_bodies, visible_columns,
    };
    use crate::domain::body::Body;

    fn body(
        name: &str,
        surface: Option<&str>,
        atmosphere: Option<&str>,
        temperature: Option<f64>,
        pressure: Option<f64>,
        gravity: Option<f64>,
        distance: Option<f64>,
    ) -> Body {
        Body {
            id64: None,
            name: name.to_owned(),
            system_id64: None,
            system_name: None,
            subtype: surface.map(str::to_owned),
            distance_ly: distance,
            is_landable: None,
            is_mapped: None,
            last_updated_at: None,
            surface_temperature: temperature,
            surface_pressure: pressure,
            gravity,
            atmosphere: atmosphere.map(str::to_owned),
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        }
    }

    #[test]
    fn optional_numbers_preserve_precision_and_units() {
        assert_eq!(format_optional(Some(190.0), 2, " K"), "190.00 K");
        assert_eq!(format_optional(Some(0.042), 3, " atm"), "0.042 atm");
        assert_eq!(format_optional(Some(0.1), 3, " g"), "0.100 g");
        assert_eq!(format_optional(Some(12.5), 2, " св. л."), "12.50 св. л.");
    }

    #[test]
    fn absent_and_non_finite_numbers_are_not_rendered_as_zero() {
        assert_eq!(format_optional(None, 2, " K"), "—");
        assert_eq!(format_optional(Some(f64::NAN), 2, " K"), "—");
        assert_eq!(format_optional(Some(f64::INFINITY), 2, " K"), "—");
    }

    #[test]
    fn sorts_all_columns_in_both_directions_and_keeps_missing_values_last() {
        let mut bodies = vec![
            body(
                "Beta",
                Some("Icy body"),
                Some("Water"),
                Some(200.0),
                Some(0.2),
                Some(0.4),
                Some(20.0),
            ),
            body("Gamma", None, None, None, None, None, None),
            body(
                "Alpha",
                Some("Rocky body"),
                Some("Ammonia"),
                Some(100.0),
                Some(0.1),
                Some(0.2),
                Some(10.0),
            ),
        ];

        for column in [
            SortColumn::Surface,
            SortColumn::Atmosphere,
            SortColumn::Temperature,
            SortColumn::Pressure,
            SortColumn::Gravity,
            SortColumn::Distance,
        ] {
            sort_bodies(&mut bodies, column, SortDirection::Ascending);
            assert_eq!(bodies.last().unwrap().name, "Gamma", "ascending {column:?}");

            sort_bodies(&mut bodies, column, SortDirection::Descending);
            assert_eq!(
                bodies.last().unwrap().name,
                "Gamma",
                "descending {column:?}"
            );
        }

        sort_bodies(&mut bodies, SortColumn::Name, SortDirection::Ascending);
        assert_eq!(
            bodies
                .iter()
                .map(|body| body.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Beta", "Gamma"]
        );
        sort_bodies(&mut bodies, SortColumn::Name, SortDirection::Descending);
        assert_eq!(
            bodies
                .iter()
                .map(|body| body.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Gamma", "Beta", "Alpha"]
        );
    }

    #[test]
    fn results_state_handles_empty_and_single_item_navigation() {
        let mut state = ResultsState::new(&[]);
        state.select_next(0);
        state.select_previous();
        assert_eq!(state.selected, 0);

        let mut state =
            ResultsState::new(&[body("Only planet", None, None, None, None, None, None)]);
        state.select_next(1);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn marked_planets_use_system_identity_when_names_repeat() {
        let mut first = body("A 1", None, None, None, None, None, None);
        first.system_id64 = Some("system-1".to_owned());
        let mut second = body("A 1", None, None, None, None, None, None);
        second.system_id64 = Some("system-2".to_owned());

        let mut state = ResultsState::new(&[first.clone(), second.clone()]);
        state.toggle_mark(&first);
        state.toggle_mark(&second);
        assert_eq!(state.marked.len(), 2);

        state.toggle_mark(&first);
        assert_eq!(state.marked.len(), 1);
        assert_eq!(body_identity(&first), "system:system-1/body:A 1");
    }

    #[test]
    fn visible_columns_adapt_to_terminal_width_without_losing_priority_fields() {
        let compact = visible_columns(LayoutMode::Compact);
        assert_eq!(
            compact,
            vec![
                SortColumn::Name,
                SortColumn::Temperature,
                SortColumn::Distance
            ]
        );

        let medium = visible_columns(LayoutMode::Medium);
        assert!(medium.contains(&SortColumn::Surface));
        assert!(medium.contains(&SortColumn::Atmosphere));
        assert!(medium.contains(&SortColumn::Temperature));
        assert!(medium.contains(&SortColumn::Distance));

        let wide = visible_columns(LayoutMode::Wide);
        assert_eq!(wide.len(), 7);
    }

    fn lines_text(lines: &[ratatui::text::Line<'static>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn summary_distinguishes_false_zero_and_unknown_values() {
        let mut planet = body("A 1", None, None, None, None, None, None);
        planet.is_mapped = Some(false);
        planet.biological_signal_count = Some(0);

        let summary = lines_text(&body_summary_lines(&planet));
        assert!(summary.contains("Картографирована: нет"));
        assert!(summary.contains("Биологические сигналы: 0"));
        assert!(summary.contains("Температура: —"));
    }

    #[test]
    fn details_do_not_invent_first_footfall_or_volcanism_status() {
        let mut planet = body("A 1", Some("Icy body"), None, None, None, None, None);
        planet.is_landable = Some(false);
        planet.biological_signal_count = Some(0);
        planet.geological_signal_count = None;
        planet.volcanism_type = None;

        let details = lines_text(&body_detail_lines(&planet));
        assert!(details.contains("Можно приземлиться: нет"));
        assert!(details.contains("Биологические сигналы Spansh: 0"));
        assert!(details.contains("Геологические сигналы Spansh: нет данных"));
        assert!(details.contains("Вулканизм: нет данных"));
        assert!(!details.contains("First Footfall"));
        assert!(!details.contains("нет вулканизма"));
    }

    #[test]
    fn details_include_known_volcanism_and_biological_genera() {
        let mut planet = body(
            "A 1",
            Some("Rocky body"),
            Some("Water"),
            Some(410.0),
            None,
            None,
            None,
        );
        planet.volcanism_type = Some("Water Magma".to_owned());
        planet.biological_genera = Some(vec!["Osseus".to_owned()]);

        let details = lines_text(&body_detail_lines(&planet));
        assert!(details.contains("Вулканизм: Water Magma"));
        assert!(details.contains("Оссии"));
    }
    #[test]
    fn result_help_lists_navigation_and_all_primary_actions() {
        let help = lines_text(&results_help_lines());
        assert!(help.contains("↑ / ↓"));
        assert!(help.contains("Space"));
        assert!(help.contains("Enter"));
        assert!(help.contains("Esc"));
        assert!(help.contains("Ctrl+Q"));
    }

    #[test]
    fn results_state_starts_with_help_closed() {
        let state = ResultsState::new(&[]);
        assert!(!state.help_open);
    }
}
