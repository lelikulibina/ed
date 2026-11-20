# SDD ledger — plan: docs/superpowers/plans/2026-11-20-spansh-full-rewrite.md

## Workspace and pre-flight

- Изолированная работа ведётся в GitHub-ветке `refactor/spansh-api-architecture`; `main` не меняется.
- В этой сессии нет локального git worktree и Rust toolchain; исходники редактируются через GitHub, проверки выполняются GitHub Actions.
- Схема Spansh не подтверждена live POST-запросом; все fixtures пока синтетические.
- Интерфейсы: Task 1 создаёт `SearchCriteria`, который потребляют Task 3 (локальная фильтрация), Task 4 (BodySearch) и Task 5 (SearchUseCase). Task 2 выдаёт `ResolvedSystem`, который потребляют Task 4 и Task 5. Task 3 создаёт `Body`, которую потребляют Task 4 и Task 5. Конфликтов типов в текущих интерфейсах не обнаружено.

## Rulings

- Ruling: использовать удалённую GitHub-ветку как изолированную рабочую область — в текущем окружении нет локального git/worktree-инструмента; цена ошибки — отсутствие локального состояния для воспроизводимых команд, компенсируется CI.
- Ruling: `-S/--system` трактуется как имя центральной системы, а не как сектор — пользователь обозначил задачу поиска вокруг указанной системы; флаг и формат аргумента сохранены, цена ошибки — прежние вызовы, передававшие только название сектора, теперь могут не разрешиться.
- Ruling: временный радиус поиска по умолчанию 100 св. л. — прежний CLI не содержит флага радиуса, а новая задача требует пространственного поиска; цена ошибки — пользователю может понадобиться иной радиус, позднее можно добавить необязательный параметр.
- Ruling: отправлять на Spansh только reference_coords и distance, остальные критерии применять локально — фактическая схема фильтров Spansh пока не подтверждена; цена ошибки — большие выдачи могут потребовать больше страниц и запросов.

## Task status

- Task 1: implementation present; CLI compatibility and domain tests added; not complete until CI test suite is green.
- Task 2: resolver, exact-match selection, right-component fallback and mock-server tests present; CI RED→GREEN observed for exact-match regression.
- Task 3: Body DTO, nested parent-system parsing, nullable fields and criteria evaluator present; not complete until CI test suite is green.
- Task 4: body search, radius check, recall pagination, deduplication, synthetic fixtures and mock-server test present; CI suite passed before the latest request-throttling change.
- Task 5: application use case, CLI wiring, grouped output and a full resolver -> bodies -> criteria mock integration test present; latest CI for the new integration test is pending.
- Task 6: legacy parser deleted and unused dependencies removed; static reference audit still required.
- Task 7: final locked CI and read-only review pending.

## Verification log

- CI run 44660914953 (commit `ca406acb107db79e244934513d022d2135f10eaf`): failed at Format. Root cause: malformed Rust string escaping in a test assertion; rewritten to parse request JSON.
- CI run 44661179563 (commit `d014441ede84ff642cac7580d0ce7397821fb2c3`): Format and Clippy passed; test failed because the mock HTTP server used double-escaped CRLF and closed before returning a valid response. Corrected the HTTP framing literals.
- CI run 44661355250 (commit `2da7435975a0c3c7e8249f8e02f0a4a2592518d1`): 21 tests passed; exact-name regression failed as expected (`Sol Alpha` selected ahead of `Sol`). This was the RED proof for exact-match selection.
- Added `exact_candidate` to prefer a case-insensitive exact name before fallback; the corrected mock test also asserts the selected ID64 is used for the detail endpoint.
- CI run 44661515323 (commit `907c49e8840b6ace4aa752fdec354c2ae1951995`): Format, Clippy and all tests passed, including the exact-name regression and paginated body-search mock server.
- CI run 44661608960 (commit `618cb9c0c15e3fc3ffb7d5062e0640051da009de`): Format, Clippy and all tests passed; generated Cargo.lock was committed as `af5da70e7179bfe1b961956196445b2369ef1951`.
- CI run 44661698868 (commit `c09c9b3142786ed0b06cc660fdaa131ef2f694d6`): Format, Clippy and all tests passed with `--locked`; confirms the committed lockfile is consistent.
- Temporary CI formatting/artifact/auto-commit steps have been removed; CI now checks the committed lockfile with `--locked`.
- Added a one-request-per-second interval to the Spansh HTTP client; CI run 44661798487 passed for this change.
- Added a full application mock integration test. CI run 44661832288 failed only at Format due to rustfmt layout in the new test; the formatting diff was applied. The latest CI runs for the formatted integration test and sibling-envelope DTO parser are pending.


## Live contract probe

- Live run 44662286671 succeeded against Spansh on 2026-11-20.
- POST `/api/bodies/search` with Sol coordinates `(0,0,0)`, radius 1 ly and page size 5 returned HTTP 2xx and five parseable body rows.
- The response did not expose a GUID/search reference recognized by the current parser. This does not prove pagination is absent; the next probe checks the top-level response keys and direct POST with `page=1`.
- Recall pagination has not yet been live-confirmed.

- Live pagination probe 44662286671 (second version) used page size 1: page 0 returned Sol (ID64 `10477373803`); direct POST with `page=1` returned a different body (ID64 `36028807496337771`). This is evidence for direct POST page pagination when no search reference is present; metadata values are being inspected in the next probe.

- The second live response exposed `count=40`, `from=0`, `size=1`, and `search_reference` containing a UUID. Direct POST with `page=1` returned `from=1` and a different body ID64. The `search_reference` field is now recognized as the recall identifier; the next live probe verifies the recall GET route.

- Live smoke 44662701767 подтвердил, что `search_reference` является рабочим recall reference: GET `/api/bodies/search/recall/{reference}/1` вернул следующую страницу.
- В `BodySearch` добавлены обе стратегии: recall по search_reference и POST с page=N, когда reference отсутствует; конец выдачи определяется через count/from, а также пустую/короткую страницу.
- Добавлены mock-тесты count/from и POST page fallback. CI run 44662932139: Format, Clippy и tests passed.
- Следующая live-проверка расширена на GET resolver `/api/search/systems?q=Sol` + `/api/system/{id64}`, чтобы проверить реальный поиск системы и координаты.
