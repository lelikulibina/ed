# План тестирования Spansh-перехода

## Принципы

- Основной CI не зависит от доступности Spansh.
- Все обычные unit/mock fixtures синтетические; live-ответы не подменяются синтетическими и их не следует называть сохранёнными fixtures.
- Сетевые smoke-тесты запускаются отдельным workflow по `workflow_dispatch` или commit marker `[spansh-live-smoke]`.
- Не загружать полный EDSM dump и не делать нагрузочные тесты Spansh.

## Unit и mock-server тесты

### CLI и доменные диапазоны
- Старые короткие/длинные флаги и значения по умолчанию.
- min/max, NaN/Infinity, count=0.
- Прежние значения planet type / atmosphere type и их aliases.

### Разрешение системы
- Exact match предпочтительнее prefix match.
- Fallback удаляет последний компонент имени по одному.
- На fallback выбирается первый результат.
- Пустой ответ и отсутствие точного совпадения.
- Координаты берутся из search result без лишнего detail GET.
- Detail fallback поддерживает `record` envelope.
- ID64 сохраняется как строка/u64 без потери точности.

### Body DTO и критерии
- Плоские поля тела и родительская система во вложенном или соседнем объекте.
- Nullable поля не превращаются в нули.
- Числовые диапазоны включают min/max.
- Landable обязателен; `is_mapped=true` исключается, `false/null` считается неотмеченным как mapped (legacy default).
- Дедупликация по body ID64; fallback использует родительскую систему и имя.
- Центральная система исключается по ID64; при отсутствии ID64 используется сравнение имени.
- Группировка строго по system_id64.

### Пагинация
- Recall по `search_reference`.
- Direct POST с `page=N`, когда reference отсутствует.
- `count/from` определяют последнюю страницу.
- Empty/short-page fallback.
- Повтор тела дедуплицируется; защитный предел страниц исключает бесконечный цикл.
- Ошибка посередине не должна превращаться в ложный успешный пустой результат.

### HTTP client
- 2xx JSON.
- 5xx и сетевые ошибки повторяются ограниченно.
- 429 имеет retry/backoff.
- Запросы ограничены concurrency=1 и интервалом 1 секунда.
- Base URL заменяется на локальный mock server.

### Полный mock-сценарий
Имя системы -> координаты -> POST body search -> recall/direct page -> нормализация -> локальные критерии -> группировка.

## Live smoke: что уже подтверждено

Workflow: `.github/workflows/spansh-live-smoke.yml`; запускается вручную или commit marker `[spansh-live-smoke]`.

Подтверждено live run `44663487447` (2026-11-20):
- `GET /api/search/systems?q=Sol` вернул `count/query/results`; запись Sol содержит `id64=10477373803`, `name=Sol`, `x/y/z=0/0/0`.
- `GET /api/system/10477373803` вернул данные системы в `record` envelope.
- Resolver успешно разрешил Sol и получил координаты `(0,0,0)`.
- `POST /api/bodies/search` с радиусом 1 ly и размером страницы 1 вернул `count=40`, `from=0`, `size=1`, `search_reference` UUID и body rows.
- `GET /api/bodies/search/recall/{search_reference}/1` вернул Mercury на следующей странице.
- `GET /api/bodies/search/recall/{search_reference}/39` вернул последнюю строку: `count=40`, `from=39`, одна строка; проверка `from + rows >= count` определила конец выдачи.
- Радиус 100 ly вернул `count=10000`; page 40 — Alpha Centauri на 4.37712 ly. В первой странице размера 100 найден landable Rocky body `2045 PC2` в Alpha Centauri.
- Размер страницы 100 работает; recall page 99 вернул `from=9900`, 100 строк и распознан как последняя страница.
- Отдельный direct POST с `page=1` также вернул следующую запись с `from=1` и другим ID64.
- Живой Mercury result содержал `id64`, `system_id64`, `subtype`, `distance`, `is_landable=true`, `is_mapped=null`, `surface_temperature`, `surface_pressure`, `gravity`, `atmosphere`. Это подтвердило форму полей, но не единицы измерения для всех типов тел.

## Что ещё не подтверждено

- Выдача 10 000 строк проверена на page size 100 до последней страницы 99. Поведение сверх 10 000 результатов и точная верхняя граница provider cap ещё не подтверждены.
- Максимальный page size и максимальное количество страниц.
- Единицы surface_temperature/surface_pressure/gravity и все enum values subtype/atmosphere.
- Distance semantics на разных центральных системах и при большом радиусе.
- 400/429/5xx, исчерпание retry, временно недоступный recall reference.
- Актуальность и полнота данных Spansh.

## CI

- `cargo fmt -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --all-targets`
- Все обычные тесты локальные и не требуют интернета.
