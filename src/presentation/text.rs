pub mod ui {
    pub const MIN_TERMINAL_SIZE: &str = "Увеличьте окно терминала до 80×24";
    pub const APP_TITLE: &str = "ED Planet Search";
    pub const HELP_TITLE: &str = " Справка ";
    pub const HELP_COMMON: &str = "? / Esc — закрыть справку\nCtrl+Q — выход";
    pub const HELP_SEARCH: &str = "УПРАВЛЕНИЕ ПОИСКОМ\n↑/↓ — выбор сценария или значения\nTab / Shift+Tab — переход между полями\nПробел — переключить флаг или поверхность\nF2 — показать/скрыть дополнительные фильтры\nHome / End — начало / конец строки\nBackspace / Delete — удалить символ\nВставка текста — вставить из буфера\nEnter — запустить поиск\n? — закрыть справку";
    pub const HELP_RUNNING: &str = "ВЫПОЛНЕНИЕ\nEsc — отменить поиск";
    pub const HELP_RESULTS: &str = "РЕЗУЛЬТАТЫ\n↑/↓ — выбрать планету · s / ы — сортировка · S / Ы — порядок\nc / с — копировать систему · e / у — изменить параметры\nn / т — новый поиск · PgUp / PgDn — подробности";
    pub const HELP_ERROR: &str = "ОШИБКА\nr / к — повторить поиск · e / у — изменить параметры\nEnter / Esc — вернуться к настройке";
    pub const SEARCH_PROFILE_TITLE: &str = " Сценарий ";
    pub const SEARCH_PARAMETERS_TITLE: &str = " Параметры ";
    pub const SEARCH_SUMMARY_TITLE: &str = " Поиск по критериям ";
    pub const SEARCH_HINTS: &str =
        "Enter — искать · Tab / Shift+Tab — поле · F2 — фильтры · ? — помощь · Esc — выход";
    pub const SEARCH_CUSTOM_PROFILE: &str = "Свой поиск";
    pub const SEARCH_SYSTEM_LABEL: &str = "Система";
    pub const SEARCH_COUNT_LABEL: &str = "Количество";
    pub const SEARCH_BIO_LABEL: &str = "Без биосигналов";
    pub const SEARCH_SURFACE_LABEL: &str = "Поверхность";
    pub const SEARCH_ATMOSPHERE_LABEL: &str = "Атмосфера";
    pub const SEARCH_TEMPERATURE_LABEL: &str = "Температура";
    pub const SEARCH_GRAVITY_LABEL: &str = "Гравитация";
    pub const SEARCH_PRESSURE_LABEL: &str = "Давление";
    pub const SEARCH_ADVANCED_LABEL: &str = "Дополнительно (F2)";
    pub const SEARCH_PROFILE_VALUE_LABEL: &str = "Потенциал профиля";
    pub const SEARCH_SUMMARY_PREFIX: &str = "Ищем";
    pub const SEARCH_RADIUS_LABEL: &str = "Радиус";
    pub const SEARCH_SPECIES_LABEL: &str = "видов";
    pub const SEARCH_SURFACE_CHOICE_LABEL: &str = "выбор";
    pub const SEARCH_PROFILE_VALUE_UNIT: &str = "млн CR";
    pub const SEARCH_RADIUS_VALUE: &str = "1000 св. лет";
    pub const FORM_ERROR_MARKER: &str = "  ✗ ";
    pub const RUNNING_TITLE: &str = " Поиск планет ";
    pub const RUNNING_PANEL_TITLE: &str = " Выполнение ";
    pub const RUNNING_SYSTEM_PENDING: &str = "Поиск центральной системы…";
    pub const RUNNING_PAGE_PENDING: &str = "Ожидание первой страницы Spansh…";
    pub const RUNNING_PREPARING: &str = "Подготовка запроса…";
    pub const RUNNING_SYSTEM_FOUND_PREFIX: &str = "Центральная система найдена";
    pub const SYSTEM_REPLACED_WARNING: &str = "ВНИМАНИЕ: точка отправления заменена";
    pub const RUNNING_PAGE_LOADED_PREFIX: &str = "Загружена страница Spansh";
    pub const RUNNING_FOUND_COUNT: &str = "Подходящих планет найдено";
    pub const RUNNING_COUNT_SEPARATOR: &str = "из";
    pub const RUNNING_CANCEL_HINT: &str = "Esc — отменить поиск";
    pub const APP_SEARCH_PREPARING: &str = "Подготовка поиска…";
    pub const APP_SEARCH_RESOLVING_SYSTEM: &str = "Поиск центральной системы (Spansh)…";
    pub const APP_SEARCH_SYSTEM_FOUND_PREFIX: &str = "Система найдена";
    pub const APP_SEARCH_PAGE_LOADED_PREFIX: &str = "Страница Spansh";
    pub const APP_SEARCH_PAGE_FOUND_SUFFIX: &str = "найдено";
    pub const APP_DEFAULT_CONFIG_ERROR: &str = "Профиль поиска по умолчанию должен быть корректным";
    pub const EVENT_SEND_ERROR: &str = "Не удалось передать событие интерфейсу";
    pub const ANIMATION_EVENT_SEND_ERROR: &str = "Не удалось передать событие анимации";
    pub const TERMINAL_RAW_MODE_ERROR: &str = "Не удалось включить raw mode";
    pub const APP_SEARCH_RETRYING: &str = "Повторный запуск поиска…";
    pub const ERROR_TITLE: &str = " Ошибка поиска ";
    pub const ERROR_HEADING: &str = "✗ Поиск не выполнен";
    pub const ERROR_FALLBACK: &str = "Не удалось выполнить поиск. Попробуйте ещё раз.";
    pub const ERROR_HINTS: &str = "r / к — повторить поиск\ne / у / Enter / Esc — изменить параметры\nCtrl+Q / Ctrl+Й — выход";
    pub const RESULTS_EMPTY_TITLE: &str = " Нет результатов ";
    pub const RESULTS_EMPTY_HINT: &str =
        "Попробуйте расширить диапазон температуры или снять ограничение по биосигналам.";
    pub const RESULTS_EMPTY_ACTIONS: &str = "e / у — изменить параметры · n / т — новый поиск";
    pub const RESULTS_HINTS: &str = "↑/↓ выбор · s/ы сортировка · S/Ы порядок · c/с копировать систему · e/у параметры · n/т новый поиск · PgUp/PgDn подробности";
    pub const RESULTS_TABLE_TITLE: &str = " Планеты ";
    pub const RESULTS_DETAILS_TITLE: &str = " Подробности ";
    pub const RESULTS_EXOBIOLOGY_TITLE: &str = " Экзобиология ";
    pub const RESULTS_EXOBIOLOGY_EMPTY: &str = "Экзобиологические кандидаты не найдены.";
    pub const RESULTS_PLANET_HEADER: &str = "Планета";
    pub const RESULTS_SYSTEM_HEADER: &str = "Система";
    pub const RESULTS_DISTANCE_HEADER: &str = "св. л.";
    pub const RESULTS_TEMPERATURE_HEADER: &str = "K";
    pub const RESULTS_PRESSURE_HEADER: &str = "atm";
    pub const RESULTS_GRAVITY_HEADER: &str = "g";
    pub const RESULTS_REVENUE_HEADER: &str = "Выручка";
    pub const RESULTS_SORT_DISTANCE: &str = "расстояние";
    pub const RESULTS_SORT_TEMPERATURE: &str = "температура";
    pub const RESULTS_SORT_PRESSURE: &str = "давление";
    pub const RESULTS_SORT_GRAVITY: &str = "гравитация";
    pub const RESULTS_SORT_NAME: &str = "название";
    pub const RESULTS_SUMMARY_LABEL: &str = "НАЙДЕНО";
    pub const RESULTS_SUMMARY_SYSTEM_LABEL: &str = "СИСТЕМА";
    pub const RESULTS_SUMMARY_SORT_LABEL: &str = "СОРТИРОВКА";
    pub const RESULTS_EMPTY_MESSAGE: &str = "Подходящие планеты не найдены";
    pub const RESULTS_EMPTY_SYSTEM_LABEL: &str = "Система";
    pub const RESULTS_DETAIL_PLANET: &str = "Планета";
    pub const RESULTS_DETAIL_SYSTEM: &str = "Система";
    pub const RESULTS_DETAIL_SURFACE: &str = "Тип поверхности";
    pub const RESULTS_DETAIL_ATMOSPHERE: &str = "Атмосфера";
    pub const RESULTS_DETAIL_TEMPERATURE: &str = "Температура";
    pub const RESULTS_DETAIL_PRESSURE: &str = "Давление";
    pub const RESULTS_DETAIL_GRAVITY: &str = "Гравитация";
    pub const RESULTS_DETAIL_DISTANCE: &str = "Расстояние";
    pub const RESULTS_DETAIL_LANDING: &str = "Посадка";
    pub const RESULTS_DETAIL_MAPPED: &str = "Картографирование";
    pub const RESULTS_DETAIL_DATA_UPDATED: &str = "Последнее обновление данных";
    pub const RESULTS_DETAIL_SIGNALS_UPDATED: &str = "Обновление сигналов";
    pub const RESULTS_DETAIL_BIO_COUNT: &str = "Биосигналы";
    pub const RESULTS_DETAIL_GEO_COUNT: &str = "Геосигналы";
    pub const RESULTS_DETAIL_BIO_GROUPS: &str = "Биологические группы";
    pub const RESULTS_DETAIL_VOLCANISM: &str = "Вулканизм";
    pub const RESULTS_EXOBIOLOGY_GROUP_HEADER: &str = "Группа";
    pub const RESULTS_EXOBIOLOGY_SPECIES_HEADER: &str = "Вид";
    pub const RESULTS_EXOBIOLOGY_VALUE_HEADER: &str = "За 3 образца";
    pub const RESULTS_EXOBIOLOGY_STATUS_HEADER: &str = "Статус";
    pub const RESULTS_MISSING_VALUE: &str = "—";
    pub const RESULTS_YES: &str = "Да";
    pub const RESULTS_NO: &str = "Нет";
    pub const COPY_STATUS_PREFIX: &str = "Скопировано";
    pub const COPY_MANUAL_FALLBACK: &str = "если OSC 52 не поддерживается, выделите имя вручную";
}

use crate::domain::{
    body::Body,
    exobiology::{self, CandidateStatus, EstimateError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExobiologyRow {
    pub genus: String,
    pub species: String,
    pub base_value_cr: u64,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExobiologyView {
    pub rows: Vec<ExobiologyRow>,
    pub summary: Vec<String>,
    pub maximum_value_cr: u64,
}

pub fn exobiology_view(body: &Body) -> Result<Option<ExobiologyView>, EstimateError> {
    let estimate = exobiology::estimate(body)?;
    if estimate.genera.is_empty() {
        return Ok(None);
    }

    let first_logged = !estimate.biological_signal_count.is_some_and(|count| count > 0);
    let multiplier = if first_logged { 5 } else { 1 };
    let rows = estimate
        .genera
        .iter()
        .flat_map(|genus| {
            genus.candidates.iter().map(move |candidate| {
                let status = match candidate.status {
                    CandidateStatus::EligibleByKnownData => {
                        "Подходит по известным данным".to_owned()
                    }
                    CandidateStatus::NeedsVerification => {
                        format!(
                            "Нужно проверить: {}",
                            candidate.unknown_conditions.join(", ")
                        )
                    }
                };
                ExobiologyRow {
                    genus: russian_group_name(&genus.genus).to_owned(),
                    species: candidate.species.clone(),
                    base_value_cr: candidate.base_value_cr * multiplier,
                    status,
                }
            })
        })
        .collect::<Vec<_>>();

    let maximum_value_cr = estimate.maximum_base_value_cr * multiplier;
    let summary = vec![if first_logged {
        format!(
            "Ожидаемая выручка (бонус ×5): {} CR",
            format_credits(maximum_value_cr)
        )
    } else {
        format!(
            "Ожидаемая выручка: {} CR",
            format_credits(maximum_value_cr)
        )
    }];

    Ok(Some(ExobiologyView {
        rows,
        summary,
        maximum_value_cr,
    }))
}

pub(crate) fn russian_group_name(name: &str) -> &str {
    match name {
        "Aleoida" => "Алеоиды",
        "Bacterium" => "Бактерии",
        "Cactoida" => "Кактусоиды",
        "Clypeus" => "Клипеусы",
        "Concha" => "Моллюски",
        "Fonticulua" => "Фонтикулы",
        "Frutexa" => "Кустарники",
        "Fumerola" => "Дымники",
        "Fungoida" => "Фунгоиды",
        "Osseus" => "Оссии",
        "Recepta" => "Рецептоиды",
        "Stratum" => "Стратум",
        "Tubus" => "Тубусы",
        "Tussock" => "Дерновина",
        other => other,
    }
}

pub(crate) fn format_credits(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(' ');
        }
        formatted.push(character);
    }
    formatted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> Body {
        Body {
            id64: Some("100".to_owned()),
            name: "Test body".to_owned(),
            system_id64: Some("200".to_owned()),
            system_name: Some("Test system".to_owned()),
            subtype: Some("Rocky body".to_owned()),
            distance_ly: Some(12.0),
            is_landable: Some(true),
            is_mapped: Some(false),
            last_updated_at: None,
            surface_temperature: Some(200.0),
            surface_pressure: Some(0.5),
            gravity: Some(0.2),
            atmosphere: Some("Water".to_owned()),
            volcanism_type: None,
            biological_signal_count: None,
            geological_signal_count: None,
            biological_genera: None,
            signals_updated_at: None,
        }
    }

    #[test]
    fn exobiology_view_preserves_candidate_values_and_statuses() {
        let view = exobiology_view(&body()).unwrap().unwrap();

        assert!(!view.rows.is_empty());
        assert!(view.rows.iter().any(|row| row.genus == "Оссии"));
        assert!(view.rows.iter().any(|row| row.species == "Discus"));
        assert!(
            view.rows
                .iter()
                .any(|row| row.status.starts_with("Нужно проверить:"))
        );
        assert!(view.maximum_value_cr > 0);
        assert!(view.summary[0].starts_with("Ожидаемая выручка (бонус ×5):"));
    }

    #[test]
    fn confirmed_spansh_genera_limit_the_exobiology_rows() {
        let mut body = body();
        body.biological_signal_count = Some(2);
        body.geological_signal_count = Some(1);
        body.biological_genera = Some(vec!["Osseus".to_owned(), "Tussock".to_owned()]);

        let view = exobiology_view(&body).unwrap().unwrap();
        assert!(
            view.rows
                .iter()
                .all(|row| row.genus == "Оссии" || row.genus == "Дерновина")
        );
        assert!(!view.rows.iter().any(|row| row.genus == "Бактерии"));
    }

    #[test]
    fn existing_biological_signals_keep_standard_reward_without_multiplier() {
        let mut body = body();
        body.biological_signal_count = Some(2);

        let view = exobiology_view(&body).unwrap().unwrap();
        assert_eq!(view.summary[0], format!("Ожидаемая выручка: {} CR", format_credits(base)));
        let base = exobiology::estimate(&body).unwrap().maximum_base_value_cr;
        assert_eq!(view.maximum_value_cr, base);
    }

    #[test]
    fn missing_biological_signal_count_assumes_first_logged_bonus() {
        let view = exobiology_view(&body()).unwrap().unwrap();
        assert!(view.summary[0].starts_with("Ожидаемая выручка (бонус ×5):"));
        let base = exobiology::estimate(&body()).unwrap().maximum_base_value_cr;
        assert_eq!(view.maximum_value_cr, base * 5);
    }

    #[test]
    fn zero_biological_signals_show_first_logged_values_for_candidates() {
        let mut body = body();
        body.biological_signal_count = Some(0);

        let view = exobiology_view(&body).unwrap().unwrap();
        assert!(!view.rows.is_empty());
        assert!(view.summary[0].starts_with("Ожидаемая выручка (бонус ×5):"));
        assert!(view.maximum_value_cr > 0);
    }

    #[test]
    fn no_candidates_without_known_genera_returns_none() {
        let mut body = body();
        body.subtype = Some("Icy body".to_owned());
        body.atmosphere = Some("Neon".to_owned());
        body.surface_temperature = Some(40.0);
        body.gravity = Some(1.0);
        body.volcanism_type = Some("No volcanism".to_owned());
        body.geological_signal_count = Some(0);

        assert!(exobiology_view(&body).unwrap().is_none());
    }

    #[test]
    fn display_helpers_localize_groups_and_format_credits() {
        assert_eq!(russian_group_name("Osseus"), "Оссии");
        assert_eq!(russian_group_name("Bacterium"), "Бактерии");
        assert_eq!(format_credits(1_234_567), "1 234 567");
    }
}
