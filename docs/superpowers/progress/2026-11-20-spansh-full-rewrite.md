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
- Task 2: resolver and fallback mock-server test present; not complete until CI test suite is green.
- Task 3: Body DTO, nested parent-system parsing, nullable fields and criteria evaluator present; not complete until CI test suite is green.
- Task 4: body search, radius check, recall pagination, deduplication, fixtures and mock-server test present; not complete until CI test suite is green.
- Task 5: application use case, CLI wiring and grouped output present; not complete until CI test suite is green.
- Task 6: legacy parser deleted and unused dependencies removed; static reference audit still required.
- Task 7: pending.

## Verification log

- CI run 44660914953 (commit `ca406acb107db79e244934513d022d2135f10eaf`): failed at Format. Root cause from logs: malformed Rust string escaping in a test assertion.
- Test assertion was rewritten to parse the serialized request body as JSON.
- CI run 44660983695 (commit `ea541e746cca4e0b924da682c39b09d9d610a2b5`): failed at Format only; rustfmt diff identified formatting changes across application, CLI, domain, Spansh query/resolver and body-search modules. Clippy and tests were skipped.
- A temporary CI artifact step now captures rustfmt-normalized `src/` so the exact formatted files can be copied back to the branch; remove this diagnostic step after formatting is committed.
- TDD RED→GREEN has not yet been demonstrated for the current feature tests. Do not claim tests pass until the latest full CI run provides that evidence.
