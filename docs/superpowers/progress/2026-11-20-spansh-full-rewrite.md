# SDD ledger — plan: docs/superpowers/plans/2026-11-20-spansh-full-rewrite.md

## Workspace and pre-flight

- Изолированная работа ведётся в GitHub-ветке `refactor/spansh-api-architecture`; `main` не меняется.
- В этой сессии нет локального git worktree и Rust toolchain; исходники редактируются через GitHub, проверки выполняются GitHub Actions.
- Схема Spansh не подтверждена live POST-запросом; все fixtures пока синтетические.
- Интерфейсы: Task 1 создаёт `SearchCriteria`, который потребляют Task 3 (локальная фильтрация), Task 4 (BodySearch) и Task 5 (SearchUseCase). Task 2 выдаёт `ResolvedSystem`, который потребляют Task 4 и Task 5. Task 3 создаёт `Body`, которую потребляют Task 4 и Task 5. Конфликтов типов в текущих интерфейсах не обнаружено.

## Rulings

- Ruling: использовать удалённую GitHub-ветку как изолированную рабочую область — в текущем окружении нет локального git/worktree-инструмента; цена ошибки — отсутствие локального состояния для воспроизводимых команд, компенсируется CI.
- Ruling: `-S/--system` трактуется как имя центральной системы, а не как сектор — пользователь обозначил задачу поиска вокруг указанной системы; флаг и формат аргумента сохранены, цена ошибки — прежние вызовы, передававшие только название сектора, теперь могут не разрешиться.
- Ruling: радиус поиска по умолчанию установлен в 1000 св. л. по уточнению пользователя — прежний CLI не содержит флага радиуса; цена ошибки — плотные области могут упереться в ограничение выдачи Spansh.
- Ruling: если Spansh не знает точное имя системы, допускается первое разрешимое совпадение после удаления компонентов имени справа — пользователь подтвердил, что исходные имена корректны, а отсутствие точной записи в Spansh нормально; цена ошибки — центр поиска может сместиться, поэтому CLI обязан явно вывести фактическую систему и предупреждение.
- Ruling: при выдаче count >= 10 000 и исчерпании страниц без достижения --count возвращать явную ошибку потенциальной неполноты, а не partial success — 10 000 наблюдалось как возможный provider cap; цена ошибки — выборка ровно из 10 000 записей тоже будет помечена как потенциально неполная.
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

- Live probe with system resolver failed at `ResolveError::InvalidResponse` for `Sol`; body-search POST and recall are already verified. This failure may be in the system search response envelope or system detail coordinate shape. Added a diagnostic live probe to inspect both GET responses before changing the resolver parser.

- Live system-search endpoint returns `{count, query, results}`; each result includes `id64`, `name`, and flat `x/y/z` coordinates. Detail endpoint returns the system under `record`. Resolver now uses coordinates from search results directly and falls back to the detail `record` envelope only when needed.
- Added RED→GREEN test coverage for the actual `record` detail envelope and a no-extra-detail-request resolver path; live resolver recheck is queued.


## Последняя проверка и live-результаты

- Live run `44663487447` успешно прошёл полный smoke: GET system search, GET system detail, resolver, POST body search и recall page 1.
- Реальный search result Sol: `id64=10477373803`, `name=Sol`, `x/y/z=0/0/0`; resolver использует эти координаты без detail GET.
- Реальный Mercury row: `id64=36028807496337771`, `system_id64=10477373803`, `subtype=Metal-rich body`, `is_landable=true`, `is_mapped=null`, температура/давление/гравитация/атмосфера присутствуют. Большой ID64 успешно преобразован в строку без потери точности.
- На основании live `is_mapped=null` критерий изменён: исключается только `Some(true)`; null трактуется как false по legacy default. Добавлен тест; CI run `44663617171` прошёл.
- CI run `44663793602`: Format, Clippy и все тесты прошли после последней правки live smoke logging.
- Live smoke workflow ограничен 10 минутами и теперь явно gated: запускается по workflow_dispatch или commit marker `[spansh-live-smoke]`; обычные commits пропускают live job.

- Full live smoke run `44663487447` прошёл: system resolver получил Sol из live search result; body POST и recall page 1 успешно обработаны; Mercury поля распарсены. `is_mapped=null` теперь соответствует legacy default false.
- Добавлен дополнительный live check прямого чтения recall page 39 для выдачи с `count=40`, чтобы подтвердить последнюю страницу и окончание по `from+rows>=count`.

- Live run `44663976650` прошёл: resolver Sol, body search, recall page 1 и recall page 39. Последняя страница вернула `count=40/from=39/rows=1`; `is_last_page` определил конец корректно.
- CI run `44663966404`: Format, Clippy и Tests passed после добавления live last-page probe.
- Большие выдачи 100+ строк, units физических полей и ошибки 400/429/5xx остаются следующими проверками.

- Live run `44663976650` прошёл проверку последней страницы: count=40, from=39, rows=1. Добавлен следующий smoke для радиуса 100 ly: проверить, что выдача выходит за 40 тел Sol и page 40 содержит тело другой системы с положительной дистанцией.

- Live 100-ly probe passed: `count=10000`; recall page 40 contained Alpha Centauri (system ID64 `1178708478315`) at 4.377120 ly from Sol. This validates the core nearby-system discovery path.
- Added one final live probe for production page size 100 and the last page of the 10,000-row response (expected page index 99 / from 9900).

- По живому 100-ly результату подтверждено, что первые 40 тел принадлежат Sol, а page 40 — Alpha Centauri (4.377 ly). В production search добавлено исключение центральной системы по ID64 с fallback по имени; mock-тест теперь проверяет, что подходящее тело Sol пропускается и поиск продолжается до соседней системы.
- Live smoke дополнен проверкой, что первая страница размера 100 содержит landable body из системы, отличной от центра.

- Live run `44664397995` прошёл end-to-end probe для 100 ly: `count=10000`; page 40 — Alpha Centauri на 4.37712 ly; первая page size100 содержит landable Rocky body `2045 PC2` в Alpha Centauri; page99 вернула `from=9900/rows=100` и распознана как последняя.
- Центральная система теперь исключается в production BodySearch по ID64 (fallback — имя). Mock fallback test специально содержит подходящее тело Sol на page0 и подтверждает, что поиск продолжает выдачу до другой системы.
- CI после добавления 100-row live probe требует только форматирования; rustfmt diff применён. Финальный CI после этих правок ожидается.

- CI run `44664493252` (commit `1197165bdca8b6abe426d4010fa7f823e433c561`): Format, Clippy and all tests passed after central-system exclusion and live page-size-100 test were added.
- Live run `44664397995` (commit `fa194da66ddc3836c94d860e053b5973adadd0fe`): 100-ly query returned count=10000; page 40 Alpha Centauri; page 0 size100 contained landable Rocky body 2045 PC2 in Alpha Centauri; page 99 returned from=9900/100 rows and was correctly recognized as last page.
- Остаточные проверки: units полей на разных типах тел, 400/429/5xx/retry exhaustion, поведение выше 10k (provider cap), и финальный независимый review.

- Добавлен дополнительный live probe страниц Sol 2/3/4 для проверки реальных значений surface_temperature/surface_pressure/gravity и сопоставления единиц измерения с legacy CLI ranges.

- Live Sol planet inspection confirmed temperature/pressure/gravity values: Venus 735 K / 93.1895 pressure / 0.9044 g; Earth 288 K / 0.9991 pressure / 0.9991 g. A broader sample of Sol pages 2-10 is being inspected to normalize atmosphere labels and confirm landability fields.

- Sol page sample shows pressure values 93.19 (Venus), 0.999 (Earth), 2.303 (Mars), gravity ~0.90/0.999/0.377 and temperatures 735/288/261. Pressure appears atm and gravity g, but broader data consistency still needs review. Added raw atmosphere field inspection to distinguish `atmosphere` from `atmosphere_type`.

- Live raw body row keys include `atmosphere_composition` and numeric `atmosphere_type`; current parser was reading only descriptive `atmosphere`. Added a probe to inspect composition values so local CLI atmosphere filters can use the correct field.

- Raw live atmosphere data: `atmosphere` is a descriptive label; `atmosphere_composition` is an array of `{name, share}`. Example Venus: CO2 96.5%; Earth: Nitrogen 77.89%, Oxygen 20.89%, Argon 0.93%; Io: Sulphur dioxide 90%.
- Body normalizer now prefers a meaningful descriptive label, but when label is `No atmosphere` derives the CLI atmosphere category from the dominant composition. Added fixture tests and live assertions for Venus=Carbon dioxide, Earth=Nitrogen, Io=Sulphur dioxide.

- Live run `44665360265` прошёл atmosphere assertions: Venus -> Carbon dioxide (label), Earth -> Nitrogen (dominant composition), Io -> Sulphur dioxide (dominant composition). Raw `atmosphere_composition` is an array of gas name/share; `atmosphere_type` is absent in these rows.
- Temperature values look like Kelvin and gravity like g; pressure units remain provisional (likely atm) and require a broader consistency check.
- CI run `44665347638` caught Clippy `collapsible_if`; corrected with a let-chain. New CI is required to confirm the fix.

- Добавлен последний end-to-end live assertion: на 100-ly page 0 ищется не центральная система, а реальная landable планета, удовлетворяющая всем широким числовым критериям и atmosphere=any.

- Live run `44665588127` прошёл end-to-end filter assertion: на 100-ly выдаче найден landable Rocky body `2045 PC2` в Alpha Centauri (4.37712 ly), temperature 85.062 K, pressure 0, gravity 0.0868 g, atmosphere No atmosphere; body прошёл широкие локальные критерии и центральная Sol была исключена.
- CI run `44665588162`: Format, Clippy и Tests passed after atmosphere composition normalization and live assertion changes.

- Дополнительный review resolver выявил edge case: при prefix fallback первым мог прийти autocomplete/name-only элемент без ID64, из-за чего поиск завершался InvalidResponse, не рассматривая следующие кандидаты. Resolver теперь пропускает записи без ID64; добавлены unit tests для exact и fallback candidate selection.
- Добавлен mock test на исчерпание четырёх попыток при постоянном HTTP 503; ожидается явная SpanshError::Status, а не пустой результат.
- CI run `44666537004` обнаружил только rustfmt diff; исправлено. CI для последнего SHA `b897773d260240dba0fb5364835880afbbeef44e` выполняется.

- Последний rustfmt review потребовал убрать пустые строки между unit tests в `client.rs` и `system.rs`; поправлено в SHA `07670204c88c8efc3da11c8874b340bb20a26cbd`. Предыдущие CI падения были только на Format и до Clippy/Tests не доходили; ожидаем новый полный прогон.

- CI `44666723736` подтвердил Format=pass, затем обнаружил два Clippy `collapsible_if` в новой логике пропуска name-only кандидатов. Оба исправлены let-chain в SHA `b9500b37c27a45a1fbca080c4ed4832590ad9df3`; новый Rust CI запущен.
- Live smoke run `44666765601` запущен на коммите с обновлённым resolver; ожидается завершение contract probe.

- CI run `44666840239` на SHA `e104df65de65bf7251e001556a9e93f56c6b7fcd`: Format, Clippy и Tests прошли.
- Live smoke run `44666765601` на resolver fix: PASS. Подтверждены Sol coordinates, recall pagination, Alpha Centauri 4.37712 ly, найденный matching body `2045 PC2`, count=10000 и последняя page 99.
- Добавил coverage test для каждого прежнего значения planet/atmosphere CLI enum. Также усилил live smoke: Venus должна возвращать ~93.19 pressure / 735 K / 0.904 g, Earth ~0.999 pressure / 288 K / 0.999 g. Это согласуется с атмосферными pressure units (atm), но остаётся проверкой на примерах, а не официальной спецификацией единиц.

- Полный enum compatibility test выявил расхождение старого парсера и CLI help: help показывал `sulphur-dioxide`, но старый `AtmosphereType::from_str` принимал только `sulphur dioxide` / `sulfur dioxide` / `sulfurdioxide`. Добавлена поддержка обеих дефисных форм без удаления старых aliases; повторная проверка CI запущена.

- После расширения enum regression test обнаружено, что прежний parser не принимал дефисную форму `sulphur-dioxide`, хотя CLI help её документировал. Добавлена совместимая поддержка дефисной формы; CI run `44667116048` прошёл Format, Clippy и Tests.
- Добавлен отдельный mock test, проверяющий, что достижение лимита страниц возвращает `BodySearchError::PageLimit`, а не маскирует неполную выдачу как успешный пустой результат. CI для SHA `d5a146645aabb3441468d61dffbb4d7206dfad2c` выполняется.

- Rust CI run `44667250884` на SHA `9bbeeac8ffc97243a106e6cfdb6d780281f13679`: Format, Clippy и Tests PASS; `40 passed, 0 failed, 1 ignored`. Включает полный набор прежних CLI enum значений, retry exhaustion и page-limit error tests.
- Запускаю повторный live smoke с усиленными проверками физических значений Venus/Earth (давление, температура, гравитация).

- Rust CI run `44667317592` на SHA `f65e8106a33d56fca3e64fa0d3d48ccf4454f2b3`: Format, Clippy, Tests PASS.
- Повторный live smoke `44667317582` PASS на том же SHA: Venus assertions прошли (735 K, 93.1895 pressure, 0.9044 g, Carbon dioxide); Earth assertions прошли (288 K, 0.99908 pressure, 0.99906 g, Nitrogen). 100-ly поиск по-прежнему находит `2045 PC2` в Alpha Centauri, последняя страница 99 из 10 000 строк определяется корректно.

- Добавлен fallback для истёкшего/временно недоступного recall: на HTTP 404/405/410/5xx поиск продолжает страницы через live-confirmed POST `page=N`; 429 и остальные ошибки по-прежнему возвращаются явно. Mock test проверяет цепочку POST page 0 -> recall 404 -> POST page 1 -> найденный результат.
- Rust CI run `44667716901` на SHA `dceeb52c87d976b6f8b8e3167d54eb78974b5716`: Format, Clippy, Tests PASS; `41 passed, 0 failed, 1 ignored`.


## Продолжение после пользовательских уточнений (2026-11-20)

- Пользователь зафиксировал радиус поиска 1000 св. лет и тестовые имена `Thraikuae NS-U e2-2`, `Boeft QX-B d951`; повторяющееся первое имя дедуплицируется, live smoke остаётся максимум два центра.
- Пользователь подтвердил, что Spansh может не знать точное имя; fallback на приближённое совпадение принят. Live smoke больше не требует точного имени, а логирует исходное и фактически разрешённое имя. CLI выводит предупреждение при расхождении.
- RED: CI run `44670479597` дошёл до теста `reports_possible_provider_truncation_when_10000_rows_are_exhausted`; 43 passed, 1 failed, 2 ignored. Тест показал, что исчерпание count=10000 возвращалось как успешный пустой результат.
- GREEN: добавлен `BodySearchError::PotentialTruncation(usize)`; при count >= 10000 и завершении выдачи до набора требуемого количества совпадений BodySearch возвращает эту ошибку. Если --count достигнут раньше, возвращаются ближайшие найденные совпадения, поскольку сортировка запроса distance ASC.
- CI run `44670644945` на SHA `e669a59e09f51599a6d26f6c2c7c10a587670281`: Format PASS, Clippy PASS, Tests PASS — 44 passed, 0 failed, 2 ignored.
- Не запускать live smoke повторно без изменения resolver/body-search контракта. Проблема покрытия сверх 10 000 остаётся открытой: пока приложение честно сообщает о возможной неполноте, но не пытается делать пространственное разбиение из-за риска большого количества запросов и отсутствия подтверждённой схемы координатных фильтров.


## Self-review после пользовательских корректировок

- Повторно просмотрены критические модули: `main.rs`, `application/search.rs`, `spansh/system.rs`, `spansh/query.rs`, `spansh/bodies.rs`, `spansh/client.rs`, `domain/body.rs`, `domain/criteria.rs`, `presentation/output.rs`.
- Проверено, что approximate resolver фактическое имя/ID64 логирует, а CLI предупреждает при отличии от введённого имени. Live smoke больше не блокируется на отсутствии exact name.
- Проверено, что поиск сортирует по distance ASC, фильтрует локально, исключает центральную систему, дедуплицирует тела и ограничивает API одним запросом за раз с интервалом не менее секунды.
- Важное открытое ограничение: нет доказанного способа получить тела за пределом наблюдавшейся выдачи 10 000 без потенциально большого количества запросов. Текущая защита не выдаёт молча неполный результат, если нужное число совпадений не набралось; пространственное разбиение не реализовано.
- Minor (deferred): фактическая форма ответа нового autocomplete endpoint после переключения resolver не перепроверялась live; повторный вызов отложен во избежание лишней нагрузки на Spansh. Mock contract tests остаются зелёными.
- Внешний review subagent недоступен; выполнен self-review автором. Он слабее независимой проверки и не заменяет review перед merge.
- Ветка на момент сравнения была на 4 коммита позади обновившегося `main`; rebase/merge не выполнялся, чтобы не затягивать несвязанные изменения и не менять базовую ветку без запроса пользователя.
