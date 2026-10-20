# Как обновлять Handy Listens

Handy Listens = Handy ([cjpais/Handy](https://github.com/cjpais/Handy)) + live captions из [vscg/Handy `feat/live-captions`](https://github.com/vscg/Handy/tree/feat/live-captions) + выборочные изменения из [egsok/klava-nevinovata](https://github.com/egsok/klava-nevinovata) + собственные функции форка.

Текущие базы записаны в [`.github/upstream-baselines.json`](.github/upstream-baselines.json): **Handy v0.9.8**, **Klava v0.9.8-1**.

Встроенный updater выключен, endpoint upstream удалён. Не ставить поверх форка официальный установщик Handy.

## Как узнаём о новых версиях

Workflow `Check upstream releases` запускается по будням. Он сравнивает последний релиз Handy и Klava с `upstream-baselines.json` и создаёт задачу `Review … release …` с меткой `upstream-review`. Сам он ничего не переносит.

## Remotes

`origin` в рабочей копии указывает на `vscg/Handy` — **это чужой репозиторий, туда не пушим**.

```bash
git remote add fork https://github.com/antuzinski/handy-listens.git
git remote add upstream https://github.com/cjpais/Handy.git
```

Публикуем только в `fork`.

## Обновление Handy

1. Ветка от `main`: `integrate/upstream-<версия>`, тег-страховка `backup/pre-<версия>`.
2. `git fetch upstream tag v<версия> --no-tags`, затем `git merge --no-ff v<версия>`. Только merge-коммит: история форка должна оставаться видимой. Не заменять ветку на чистый upstream.
3. Конфликты решать так, чтобы сохранить функции форка (список ниже). Если Handy меняет сигнатуры, обновить вызовы и тесты форка.

## Обновление Klava

Klava сама вливает новые версии Handy, поэтому её история в основном состоит из коммитов Handy. Брать её целиком нельзя.

1. Сначала довести наш форк до той же версии Handy, на которой стоит новый релиз Klava.
2. Получить слой Klava поверх Handy: `git diff <тег Handy> <тег Klava> -- <файлы>`.
3. Сравнить его со слоем прошлой интегрированной версии (`git diff <прошлый Handy> <прошлая Klava>`). Новое — это разница между двумя слоями.
4. Нужные файлы накладывать через `git apply -3`: на общей базе Handy патч ложится чисто.

### Что из Klava уже взято (v0.9.8-1)

- атомарные настройки (`update_settings*`), безопасные значения по умолчанию для записей и истории;
- watchdog (`src-tauri/src/watchdog.rs`) и `handy-keys` из `egsok/handy-keys-fork` (rev `2a1ae0b`, просмотрен);
- координатор: классификация тапа и удержания по времени события; восстановление хука и сверка клавиши отмены;
- буфер обмена: зависший владелец не блокирует вставку, сохраняются все форматы;
- кавычки в пути автозапуска Windows.

### Что не взято и почему

- бренд, тема, иконки, README и release notes Klava;
- `transcription_prompt` (у нас свой `whisper_prompt` и пресеты);
- свой ID и миграция данных (`migration.rs`), установщик, updater — запланированы отдельным релизом, см. [`docs/upstream-review-2026-10.md`](docs/upstream-review-2026-10.md);
- обработка текста Klava (`audio_toolkit/text.rs`) — следующий шаг, этап 3 в том же документе.

## Функции форка, которые нельзя потерять

- Captions: микрофон и системный звук в одной дорожке, выбор устройства системного звука;
- Whisper: промпт и пресеты (`whisper_prompt`, `whisper_prompt_preset`), контекст 128 токенов, чистка повторов и галлюцинаций, границы слов Breeze;
- словарь `dictionary.txt` и промпт чистки `cleanup_prompt.txt` (`src-tauri/src/dictionary/`);
- каталог Whisper Turbo на 100 языков и поиск языка;
- зелёная тема и слои выпадающих меню;
- имя Handy Listens; updater выключен.

## Сборка на Windows

- `cmake` и Ninja берутся из Build Tools: `C:\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\{CMake\bin,Ninja}`.
- Vulkan SDK нужен transcribe-cpp: `VULKAN_SDK=C:\VulkanSDK\<версия>`.
- **Собирать только в отдельную папку**: `CARGO_TARGET_DIR=src-tauri\target\integrate`. Рабочий Handy пользователя запускается из `src-tauri\target\release`, и сборка поверх него заменит файлы запущенной программы.
- Тесты без видеокарты: `TRANSCRIBE_CMAKE_ARGS=-DTRANSCRIBE_VULKAN=OFF`, `CARGO_TARGET_DIR=target/novulkan`, `cargo test --lib`.
- В PATH стоит заглушка Python из Microsoft Store; для скриптов использовать node или bun.

## Проверка перед публикацией

1. `cargo test --lib`, `cargo fmt --check`, `bun run build`, `bun run lint`.
2. Релизная сборка с Vulkan в `target\integrate`, копия в отдельную папку рядом с рабочей версией.
3. Ручная проверка: диктовка (тап и удержание), Esc после короткой записи, Captions, вставка с картинкой в буфере, словарь (кнопка «Проверить»), промпт Whisper, индикатор записи, тема, 100 языков у Turbo; настройки и история на месте.
4. Обновить `.github/upstream-baselines.json`, этот файл и `PROJECT_OVERVIEW.md`; закрыть задачи `upstream-review` со списком того, что взято и что нет.

Перед первым запуском новой версии сохранить копию `%APPDATA%\com.pais.handy` при закрытом приложении. Прежнюю сборку не удалять, пока новая не проверена.
