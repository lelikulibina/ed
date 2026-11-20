# Полная замена внутреннего поиска на Spansh — план реализации

**Цель:** заменить EDSM HTML-парсер поиском через Spansh, сохранив прежние параметры CLI без изменений.

**Рабочая ветка:** `refactor/spansh-api-architecture` (создана от `main`; в main ничего не влито).

**Единственный обязательный legacy-контракт:** прежние CLI-флаги и значения по умолчанию:
- `-S/--system` — обязательное имя центральной системы;
- `-P/--planet-type` — обязательный тип поверхности;
- `-A/--atmosphere-type` — обязательный тип атмосферы;
- `-t/--temperature` — `0-5000`;
- `-g/--gravity` — `0.1-1.0`;
- `-p/--pressure` — `0.0-10.0`;
- `-c/--count` — `5`.

Внутренние модули, база EDSM и HTML scraping не являются контрактом и удалены.

## Итог реализации

### Задача 1 — CLI и доменные типы
**Статус: готово**

- [x] Сохранены старые короткие/длинные CLI-флаги и defaults.
- [x] Добавлены CLI compatibility tests.
- [x] Добавлены `SearchCriteria`, диапазоны, нормализация прежних значений enum и валидация.

### Задача 2 — Resolver центральной системы
**Статус: готово**

- [x] Exact match имени предпочтительнее prefix match.
- [x] Fallback удаляет конечные компоненты по одному; на fallback берётся первый результат.
- [x] ID64 хранится как строка/u64 без преобразования в f64.
- [x] Координаты читаются прямо из search result `id64/name/x/y/z`.
- [x] Fallback detail parser поддерживает `record` envelope.
- [x] Mock tests и live smoke для Sol прошли.

### Задача 3 — DTO тел и локальная фильтрация
**Статус: готово**

- [x] Реализован Body DTO с nullable-полями, ID64 и parent-system данными.
- [x] Поддерживаются flat body fields, вложенный body и sibling system envelope.
- [x] Фильтры subtype, atmosphere, temperature, pressure, gravity, landable.
- [x] `is_mapped=true` исключается; `false/null` считается неотмеченным как mapped по legacy default.
- [x] Дедупликация по body ID64; fallback по системе и имени.
- [x] Группировка строго по system_id64.

### Задача 4 — Body search и пагинация
**Статус: готово**

- [x] Реализован POST `/api/bodies/search` с reference_coords, distance, sort, size, page.
- [x] `search_reference` распознаётся как UUID.
- [x] Recall GET `/api/bodies/search/recall/{reference}/{page}` подтверждён live для page 1.
- [x] Direct POST с увеличенным `page` используется, если reference отсутствует; подтверждён live.
- [x] Конец выдачи определяется по `count/from`, empty/short-page fallback.
- [x] Защитный предел 100 страниц, дедупликация и mock-server tests.

### Задача 5 — SearchUseCase и вывод
**Статус: готово**

- [x] CLI подключён к новому SearchUseCase.
- [x] Полный mock-сценарий: имя -> координаты -> body search -> страницы -> фильтры -> результаты.
- [x] `--count` ограничивает количество подходящих тел, а не количество страниц.
- [x] Результаты группируются по ID64 родительской системы.

### Задача 6 — Удаление старой реализации
**Статус: готово**

- [x] Удалён `src/parser/`.
- [x] Удалены scraper/futures/indicatif/urlencoding и обновлён Cargo.lock.
- [x] Нет локальной базы, импорта dump или фонового обновления.
- [x] CLI не зависит от старого EDSM runtime.

### Задача 7 — Проверка
**Статус: базовые проверки пройдены**

- [x] `cargo fmt -- --check`.
- [x] `cargo clippy --locked --all-targets --all-features -- -D warnings`.
- [x] `cargo test --locked --all-targets`.
- [x] Проверены старые CLI flags/defaults unit tests.
- [x] Live smoke: system search, detail envelope, body search, search_reference и recall page 1.
- [ ] Проверить последнюю страницу и большой поиск (100+ результатов).
- [ ] Проверить единицы surface_pressure/gravity и все варианты subtype/atmosphere на реальных данных.
- [ ] Проверить 400/429/5xx, исчерпание retry и временную недоступность recall.
- [ ] Провести повторный review после этих проверок.

## Архитектурные решения и оставшиеся ограничения

- Локальная БД и EDSM dump не используются.
- Радиус по умолчанию — 100 св. л. (временная константа; старый CLI не имел radius-флага).
- Spansh HTTP client: concurrency=1, минимум 1 секунда между началами запросов.
- Live подтверждены Sol coordinates `(0,0,0)`, body search, response metadata `count/from/size`, `search_reference`, recall page 1 и direct POST page 1.
- Полный контракт API не считается окончательно подтверждённым: последняя страница, большие выдачи, units и все enum values ещё требуют проверки.

**Подробности:** `docs/architecture/spansh-redesign.md` и `docs/testing/spansh-test-plan.md`.
