# Handy Listens: проект и обновление

## Где лежит проект

- Репозиторий: https://github.com/antuzinski/handy-listens, ветка `main`. В рабочей копии `origin` указывает на чужой `vscg/Handy`; публиковать только в remote `fork` (см. [FORK_UPDATING.md](FORK_UPDATING.md)).
- Рабочие исходники: `C:\Users\Huawei\Documents\ChatGPT\Работа\work\Handy-live-captions`.
- Итоговые файлы: `C:\handy-fork`. В `portable` сохранена Windows-сборка на Handy 0.9.7, тестовые сборки лежат рядом в отдельных папках.
- История изменений — в `PROJECT_OVERVIEW.md`, разбор обновления октября 2026 и план следующих этапов — в [docs/upstream-review-2026-10.md](docs/upstream-review-2026-10.md), план словаря — в [docs/dictionary-cleanup-plan.md](docs/dictionary-cleanup-plan.md).

## Что сделано

Основа — Handy с веткой `vscg/Handy` `feat/live-captions`; интегрирован upstream Handy v0.9.8 и изменения klava-nevinovata v0.9.8-1 для надёжной работы горячих клавиш, буфера обмена и настроек. Captions захватывает микрофон и системный звук в одну дорожку. Есть настройки Whisper (промпт, пресеты, контекст), обработки распознавания, словарь `dictionary.txt` с заменами для любого языка и заготовка LLM-чистки речи. Зелёная тема, каталог Whisper Turbo на 100 языков.

Приложение использует React/TypeScript для интерфейса и Tauri/Rust для аудио и распознавания. Модели загружаются отдельно; модели, настройки, словарь и история хранятся в `%APPDATA%\com.pais.handy` и в репозиторий не входят.

## Как запускать и собирать

Сборка запускается через `handy.exe`; рядом нужны DLL и каталог `resources`. Копируйте комплект целиком.

Для новой сборки нужны Git, Bun, Rust, Windows C++ Build Tools, Vulkan SDK и зависимости из `BUILD.md`. В исходниках выполните `bun install`, затем сборку **в отдельную папку**:

```powershell
$env:VULKAN_SDK = "C:\VulkanSDK\1.4.363.0"
$env:CARGO_TARGET_DIR = "src-tauri\target\integrate"
bun run tauri build --no-bundle
```

Результат — `src-tauri\target\integrate\release`. Не собирайте в `src-tauri\target\release`: оттуда запускается рабочая версия. Подробности (cmake, тесты без видеокарты) — в [FORK_UPDATING.md](FORK_UPDATING.md).

## Как обновлять форк

Проверка новых выпусков Handy и Klava через GitHub Actions создаёт задачи с меткой `upstream-review`. Она не переносит чужие изменения автоматически. Порядок интеграции, список функций форка, которые нельзя потерять, и чек-лист проверки — в [FORK_UPDATING.md](FORK_UPDATING.md).

Автообновление выключено. Для его включения нужны собственный ID приложения с переносом данных, ключ подписи и manifest релизов (этапы 4–5 плана). Не включайте upstream-обновления вместо канала форка.
