# Схема данных

Все файлы — UTF-8, JSON (для `questions/*.jsonl` — по одному объекту на строку).

## `data/questions/<subject>.jsonl` — канонические задания

| поле | тип | описание |
|---|---|---|
| `id` | u32 | ID задания на сайте (уникален внутри предмета) |
| `subject` | string | слаг предмета (`mathematics`, `ukrmova`, …) |
| `tip` | u8 | внутренний тип задания сайта |
| `type` | string | `single_choice`, `short_answer`, `matching`, `multi_choice`, `sequence` |
| `out_order` | u32 | порядковый номер в исходной странице (не в тесте) |
| `text` | string | текст задания без HTML (LaTeX как есть) |
| `question_html` | string | HTML задания; формулы в `<latex>…</latex>` |
| `answers` | Answer[] | варианты ответа (маркер + HTML) |
| `groups` | Group[]? | группы вариантов для заданий на соответствие |
| `answer_table_html` | string? | таблица выбора (для matching) |
| `correct` | string[] | правильные значения: `["d"]`, `["1d","2b","3c"]`, `["-2.5"]` |
| `correct_raw` | string? | исходная строка ответа (`"1d;2b;3c"`) |
| `input` | bool | задание с полем ввода |
| `input_maxlength` | u32? | ограничение длины ввода |
| `explanation_html` | string? | разбор задания (HTML + LaTeX) |
| `explanation_text` | string? | разбор без HTML |
| `topic` | string? | тема из строки `ТЕМА:` в разборе |
| `latex` | string[] | все LaTeX-формулы задания и ответов |
| `images` | ImageRef[] | картинки: `{url, path}` |
| `source_test` | u32? | ID теста, из которого задание взято |
| `tests` | TestRef[] | где встречается: `{test_id, order}` |

### `Answer`
```json
{ "marker": "А", "html": "<latex>15</latex>" }
```

### `Group` (соответствие)
```json
{ "title": "Функція", "answers": [ { "marker": "1", "html": "..." } ] }
```

### `ImageRef`
```json
{ "url": "https://zno.osvita.ua/doc/images/...png",
  "path": "images/doc/images/...png" }
```

## `data/tests/<subject>/<id>.json` — вариант теста

| поле | тип | описание |
|---|---|---|
| `test_id` | u32 | ID теста |
| `id` | u32 | дубль `test_id` из листинга |
| `subject` | string | слаг предмета |
| `subject_name` | string | название предмета |
| `title` | string | «НМТ онлайн 2026 року з математики – 1 сесія» |
| `year` | u16? | год |
| `kind` | string | `test`, `nmt`, `demo`, `trial` |
| `session` | string? | `1`, `2`, `main`, `extra` |
| `total` | u32? | всего заданий |
| `questions_count` | u32 | распарсено заданий |
| `questions` | Question[] | задания в порядке теста |
| `url` | string | исходный URL |

`data/tests/<subject>/_index.json` — список тестов предмета.

## `data/nmt/<subject>.json`

```json
{ "subject": "mathematics",
  "variants": [ { "slug": "mathematics_nmt_tr_v48", "title": "...",
                  "url": "...", "questions_count": 22 } ] }
```

Файлы вариантов: `data/nmt/<subject>/<slug>.json` — та же структура, что и тест.

## `data/topics/<subject>.json`

```json
{ "subject": "mathematics",
  "topics": [ { "slug": "dijsni_chysla", "name": "Дійсні числа", "url": "..." } ] }
```

## `data/index.json`

```json
{ "subjects": [ { "slug": "...", "name": "...", "tests": 53,
                  "questions": 2595, "topics": 29, "nmt_variants": 50 } ],
  "tests": 0, "questions": 0 }
```

## `data/master.json`, `data/for_teachers.json`

```json
{ "sections": [ { "slug": "english", "name": "Англійська мова" } ],
  "tests": [ { "id": 630, "title": "...", "section": "english",
               "questions_count": 20 } ] }
```

## Соответствие ответов

`correct` хранит «сырые» значения сайта:

| тип | значение | смысл |
|---|---|---|
| `single_choice` | `"d"` | 4-й вариант (А=1 … Д=5) |
| `matching` | `"1d;2b;3c"` | пункт 1 → Д, 2 → Б, 3 → В |
| `short_answer` | `"-2.5"` | эталонная строка (сравнение с учётом `,`/`.`) |

Чтобы получить букву варианта: `index = ord(value) - ord('a')` → `answers[index].marker`.
