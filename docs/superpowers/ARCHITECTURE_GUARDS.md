# Реестр Архитектурных Гвардейцев (29/29)

Этот документ описывает 29 паттернов и правил, которые автоматически проверяются в проекте "Savage Fantasy" для поддержания стандарта **Absolute Zenith**.

---

## 1. Общая Целостность (Main Guards)

### Гвардеец #1: Чистый `main.rs`
*   **Паттерн**: Инкапсуляция инициализации.
*   **Правило**: `main.rs` должен содержать только запуск приложения и регистрацию плагинов. Вся логика — в модулях.
*   **Тест**: `main_guards::test_main_rs_is_clean`

### Гвардеец #2: Группировка Плагинов
*   **Паттерн**: Структурированная инициализация.
*   **Правило**: Плагины должны регистрироваться кортежами для наглядности и управления порядком загрузки.
*   **Тест**: `main_guards::test_plugins_are_grouped`

### Гвардеец #3: Стандартные Плагины
*   **Паттерн**: Конфигурация движка.
*   **Правило**: `DefaultPlugins` должны быть настроены (Window, Log) перед запуском.
*   **Тест**: `main_guards::test_default_plugins_are_configured`

### Гвардеец #4: Робастный Panic Hook
*   **Паттерн**: Безопасность окружения.
*   **Правило**: В `main()` должен быть установлен кастомный обработчик паник для логирования в CLI/Файл.
*   **Тест**: `main_guards::test_panic_hook_is_robust`

### Гвардеец #5: Условные Debug-инструменты
*   **Паттерн**: Оптимизация сборки.
*   **Правило**: Инструменты инспекции и отладки должны быть обернуты в `#[cfg(debug_assertions)]`.
*   **Тест**: `main_guards::test_debug_tools_are_conditional`

---

## 2. Дисциплина ECS (ECS Guards)

### Гвардеец #6: Все есть Плагин
*   **Паттерн**: Модульность 2.0.
*   **Правило**: Каждый файл в `src/` (кроме `main.rs` и `sets.rs`) обязан реализовывать `Plugin`.
*   **Тест**: `ecs_guards::test_all_src_files_are_plugins`

### Гвардеец #7: Реактивный Выбор (Observers)
*   **Паттерн**: Reactivity.
*   **Правило**: Логика выделения сущностей (Selection) должна использовать `Observer`, а не поллинг в `Update`.
*   **Тест**: `ecs_guards::test_pawn_selection_uses_observers`

### Гвардеец #8: Явный GameState
*   **Паттерн**: Управление состоянием.
*   **Правило**: Проект должен использовать Bevy `States` для глобального контроля (Loading, Playing).
*   **Тест**: `ecs_guards::test_game_state_is_implemented`

### Гвардеец #9: Запрет на Boolean State Flags
*   **Паттерн**: Finite State Machine.
*   **Правило**: Запрещено использовать `is_paused: bool` и аналогичные флаги. Используйте `State`.
*   **Тест**: `ecs_guards::test_no_boolean_state_flags`

### Гвардеец #10: Запрет на прямой World Mut
*   **Паттерн**: Command Decoupling.
*   **Правило**: Системы не должны напрямую мутировать `&mut World`. Исключение: реализации кастомных `Command`.
*   **Тест**: `ecs_guards::test_no_direct_world_mutation_in_systems`

### Гвардеец #11: Запрет на Boolean Marker Flags
*   **Паттерн**: Type-Driven Classification.
*   **Правило**: Запрещено использовать `bool` поля внутри компонентов для классификации (например, `is_worker: bool`). Используйте ZST-компоненты.
*   **Тест**: `ecs_guards::test_no_boolean_classification_flags`

### Гвардеец #12: Атомарность через Bundles
*   **Паттерн**: Entity Integrity.
*   **Правило**: Сложные сущности должны спавниться через именованные `Bundle`, а не через кортежи анонимных компонентов.
*   **Тест**: `ecs_guards::test_complex_entities_use_bundles`

### Гвардеец #13: Глобальное Расписание (System Sets)
*   **Паттерн**: Orchestration.
*   **Правило**: Каждая система должна принадлежать к `SystemSet` (`Logic`, `Visuals` и т.д.).
*   **Тест**: `ecs_guards::test_systems_belong_to_sets`

### Гвардеец #14: Глобальный Щит (Run Conditions)
*   **Паттерн**: Security.
*   **Правило**: Игровая логика должна быть защищена условием `in_state(GameState::Playing)`.
*   **Тест**: `ecs_guards::test_logic_systems_use_run_conditions`

### Гвардеец #15: Фильтрация Запросов (Query Filters)
*   **Паттерн**: Performance.
*   **Правило**: Мутабельные запросы (`Query<&mut T>`) обязаны иметь фильтры (`With`, `Without`, `Changed`), чтобы избежать лишних итераций.
*   **Тест**: `ecs_guards::test_queries_use_filters`

### Гвардеец #16: Отделение Ассетов от Логики
*   **Паттерн**: Asset Management.
*   **Правило**: Системы логики не должны создавать меши или материалы напрямую. Используйте `GameAssets`.
*   **Тест**: `ecs_guards::test_no_asset_creation_in_logic`

---

## 3. Поведение и Граф (Behavior & Graph)

### Гвардеец #17: Безопасный Переключатель (Safe Switcher)
*   **Паттерн**: Atomic State Transitions.
*   **Правило**: Смена поведения ИИ должна происходить через `switch_behavior API`, гарантирующий очистку старых состояний.
*   **Тест**: `behavior_guards::test_safe_behavior_transitions`

### Гвардеец #18: Запрет Сырых Entity-ссылок
*   **Паттерн**: Semantic Graph.
*   **Правило**: Запрещено хранить `Entity` ID в компонентах. Используйте Bevy 0.18 `Relations` API.
*   **Тест**: `graph_guards::test_no_raw_entity_references_in_components`

---

## 4. Качество и Стандарты (Quality & Meta)

### Гвардеец #19: Декаплинг Систем (Decoupling)
*   **Паттерн**: Silent Logic.
*   **Правило**: Код симуляции не должен использовать прямой ввод (Input) или вывод (Logging).
*   **Тест**: `decoupling::test_architectural_decoupling`

### Гвардеец #20: Отсутствие Скрытых Состояний (DI)
*   **Паттерн**: Explicit Dependency Injection.
*   **Правило**: Запрещено использование `static mut`, `OnceLock` или глобальных синглтонов для игровых данных. Все — через ECS.
*   **Тест**: `dependency::test_no_hidden_global_states`

### Гвардеец #21: Строгая Типизация Статистик
*   **Паттерн**: Type-Driven Domain.
*   **Правило**: Статы (`Hunger`, `Morale`) должны быть Opaque Newtypes с приватными полями и валидацией.
*   **Тест**: `type_safety::test_strict_type_driven_stats`

### Гвардеец #22: Нулевая Терпимость к Unwrap
*   **Паттерн**: Crash-Proof Stability.
*   **Правило**: Запрещено использование `.unwrap()` и `.expect()` в `src/`. Только безопасная обработка.
*   **Тест**: `stability::test_no_unwraps_in_production_code`

### Гвардеец #23: Лимит Длины Файла
*   **Паттерн**: Encapsulation.
*   **Правило**: Максимальная длина файла — 300 строк. Логика должна дробиться на модули.
*   **Тест**: `encapsulation::test_file_line_count_limit`

### Гвардеец #24: Гвардеец Качества (Clippy)
*   **Паттерн**: Linting Mandate.
*   **Правило**: Проект должен содержать настройки `all = "deny"` и `pedantic = "deny"` в `Cargo.toml`.
*   **Тест**: `linting::test_clippy_guards_presence`

### Гвардеец #25: Гвардеец Стиля (Formatting)
*   **Паттерн**: Uniform Code Style.
*   **Правило**: Весь код должен быть отформатирован через `cargo fmt`.
*   **Тест**: `linting::test_code_formatting`

### Гвардеец #26: Стандарт Коммитов
*   **Паттерн**: Traceable History.
*   **Правило**: Сообщения коммитов должны содержать блоки `What:` и `Why:`.
*   **Тест**: `metadata::test_commit_message_standard`

### Гвардеец #27: Запрет Анонимных Очередей (Plugins 2.0)
*   **Паттерн**: Debuggable Command Stream.
*   **Правило**: Запрещено использование анонимных замыканий в `.queue()`. Используйте именованные структуры `Command`.
*   **Тест**: `plugins::test_no_anonymous_command_queues`

---

## 5. Производительность (Performance)

### Гвардеец #28: Реактивное Управление Маркерами
*   **Паттерн**: Reactive Performance.
*   **Правило**: Манипуляция ZST-маркерами (состояниями) должна быть реактивной или использовать `Added`/`Removed` фильтры.
*   **Тест**: `performance::test_global_performance_guards`

### Гвардеец #29: Разделение Симуляции и Презентации
*   **Паттерн**: Determinism.
*   **Правило**: Симуляция (Логика) должна жить в `FixedUpdate`, визуализация — в `Update`.
*   **Тест**: `performance::test_simulation_presentation_split`
