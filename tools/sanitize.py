"""
sanitize.py — собирает публичную копию датасета ЗНО/НМТ без чужих разборов.

Что делает:
  * читает data/questions/<subject>.jsonl и data/tests/<subject>/*.json;
  * удаляет поля explanation_html и explanation_text;
  * оставляет в images[] только картинки, которые используются в официальных
    частях задания (question_html, answers, groups, answer_table_html);
    картинки, встречающиеся только в разборах, отбрасываются;
  * пишет копию в public/ с той же структурой;
  * пишет public/MANIFEST.json и public/LICENSE-NOTICE.md.

Флаги:
  --subjects mathematics,ukrmova   только указанные предметы (по умолчанию все)
  --keep-explanations              ничего не вырезать (копия "как есть")
  --src PATH                       корень датасета (по умолчанию — родитель tools/)
  --out PATH                       куда писать (по умолчанию <src>/public)
  --copy-images                    скопировать только используемые картинки

Примеры:
  python tools/sanitize.py
  python tools/sanitize.py --subjects mathematics,ukrmova
  python tools/sanitize.py --keep-explanations --out public-full
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlsplit

EXPLANATION_FIELDS = ("explanation_html", "explanation_text")
IMG_RE = re.compile(r"""<img\b[^>]*?\bsrc\s*=\s*["']([^"']+)["']""", re.IGNORECASE)
LICENSE_FILE = "LICENSE-NOTICE.md"
MANIFEST_FILE = "MANIFEST.json"
SOURCE_SITE = "https://zno.osvita.ua"

LICENSE_HEAD = """# Лицензионная заметка

## Что можно публиковать

- **Официальные задания ЗНО/НМТ и эталонные ответы** — это официальные материалы
  Украинского центра оценивания качества образования (УЦОЯО), которые создаются
  и публикуются государством.
- **Структуру данных и метаданные**: предмет, год, сессия, порядок заданий,
  идентификаторы, типы заданий, правильные ответы.
- **Собственные разборы, подсказки и оформление**, созданные вами.

## Источник данных

Датасет собран с публичных страниц <https://zno.osvita.ua> (проект «Освіта.ua»)
для личного использования и разработки. При публикации приложения укажите источник
и не выдавайте материалы за свои.
"""

LICENSE_EXCLUDED = """
## Что исключено из этой копии

- Поля `explanation_html` и `explanation_text` **удалены**.
- Изображения, которые встречаются только в разборах и отсутствуют в самих
  заданиях, **удалены** из массива `images`.
- Разборы и их оформление — авторский контент проекта «Освіта.ua»
  (<https://zno.osvita.ua>) — вместе с этим датасетом не распространяются.
"""

LICENSE_KEPT = """
## ВНИМАНИЕ: разборы сохранены

Сборка создана с флагом `--keep-explanations` и содержит поля
`explanation_html` / `explanation_text`, а также картинки из разборов.
Такая сборка предназначена **только для личного использования** и
**не должна публиковаться**.
"""

LICENSE_TAIL = """
## Оговорка

Эта заметка не является юридической консультацией. Перед коммерческой
публикацией проверьте условия использования первоисточника и при необходимости
заключите договор с правообладателем.
"""

def build_license_notice(keep_explanations: bool) -> str:
    body = LICENSE_KEPT if keep_explanations else LICENSE_EXCLUDED
    return LICENSE_HEAD + body + LICENSE_TAIL

def log(message: str) -> None:
    print(message, flush=True)

def image_key(value) -> str:
    """Нормализует URL/путь картинки до вида /doc/images/... (или /images/...)."""
    s = str(value or "").strip()
    if not s:
        return ""
    if "://" in s:
        s = urlsplit(s).path
    s = s.split("?", 1)[0].split("#", 1)[0]

    idx = s.find("/doc/")
    if idx >= 0:
        return s[idx:]

    idx = s.find("doc/images/")
    if idx >= 0:
        return "/" + s[idx:]

    idx = s.find("images/")
    if idx >= 0:
        return "/" + s[idx:]

    return s

def image_ref_key(ref) -> str:
    if isinstance(ref, dict):
        return image_key(ref.get("url") or ref.get("path") or "")
    return image_key(ref)

def html_image_keys(html) -> set[str]:
    if not html:
        return set()
    return {image_key(m.group(1)) for m in IMG_RE.finditer(html)}

def official_image_keys(question: dict) -> set[str]:
    """Картинки официальной части задания (без разбора)."""
    keys = html_image_keys(question.get("question_html"))
    keys |= html_image_keys(question.get("answer_table_html"))

    for answer in question.get("answers") or []:
        if isinstance(answer, dict):
            keys |= html_image_keys(answer.get("html"))

    for group in question.get("groups") or []:
        if isinstance(group, dict):
            for answer in group.get("answers") or []:
                if isinstance(answer, dict):
                    keys |= html_image_keys(answer.get("html"))

    keys.discard("")
    return keys

def sanitize_question(question: dict, keep_explanations: bool):
    """Возвращает (очищенное задание, сколько полей разбора удалено, сколько картинок удалено)."""
    if keep_explanations:
        return json.loads(json.dumps(question)), 0, 0

    out = dict(question)

    removed_fields = 0
    for field in EXPLANATION_FIELDS:
        if field in out:
            out.pop(field, None)
            removed_fields += 1

    official = official_image_keys(question)
    official_names = {Path(key).name for key in official}

    removed_images = 0
    refs = out.get("images")
    if isinstance(refs, list):
        kept = []
        for ref in refs:
            key = image_ref_key(ref)
            if key and (key in official or Path(key).name in official_names):
                kept.append(ref)
            else:
                removed_images += 1
        out["images"] = kept

    return out, removed_fields, removed_images

def sanitize_test(test: dict, keep_explanations: bool):
    out = {key: value for key, value in test.items() if key != "questions"}
    questions = []
    removed_fields = 0
    removed_images = 0
    for question in test.get("questions") or []:
        clean, fields, images = sanitize_question(question, keep_explanations)
        questions.append(clean)
        removed_fields += fields
        removed_images += images
    out["questions"] = questions
    return out, removed_fields, removed_images

def split_subjects(raw: str | None) -> list[str] | None:
    if not raw:
        return None
    parts = [part for chunk in raw.split(",") for part in chunk.split()]
    subjects = []
    for part in parts:
        part = part.strip()
        if part and part not in subjects:
            subjects.append(part)
    return subjects

def discover_subjects(data_dir: Path) -> list[str]:
    subjects = set()
    questions_dir = data_dir / "questions"
    if questions_dir.is_dir():
        subjects.update(path.stem for path in questions_dir.glob("*.jsonl"))
    tests_dir = data_dir / "tests"
    if tests_dir.is_dir():
        subjects.update(path.name for path in tests_dir.iterdir() if path.is_dir())
    return sorted(subjects)

def write_json(path: Path, payload) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as fh:
        json.dump(payload, fh, ensure_ascii=False, indent=1)
        fh.write("\n")

def process_questions_file(src: Path, dst: Path, keep_explanations: bool) -> dict:
    stats = {
        "questions": 0,
        "skipped_lines": 0,
        "explanations_removed": 0,
        "images_removed": 0,
    }
    dst.parent.mkdir(parents=True, exist_ok=True)

    with src.open("r", encoding="utf-8", errors="replace") as fin, dst.open(
        "w", encoding="utf-8", newline="\n"
    ) as fout:
        for lineno, line in enumerate(fin, 1):
            line = line.strip()
            if not line:
                continue
            try:
                question = json.loads(line)
            except json.JSONDecodeError as exc:
                stats["skipped_lines"] += 1
                log(f"  ! {src.name}:{lineno}: пропущена строка ({exc.msg})")
                continue
            if not isinstance(question, dict):
                stats["skipped_lines"] += 1
                log(f"  ! {src.name}:{lineno}: пропущена строка (не объект JSON)")
                continue

            clean, fields, images = sanitize_question(question, keep_explanations)
            fout.write(json.dumps(clean, ensure_ascii=False) + "\n")
            stats["questions"] += 1
            stats["explanations_removed"] += fields
            stats["images_removed"] += images

    return stats

def process_tests_dir(
    src_dir: Path, dst_dir: Path, keep_explanations: bool
) -> dict:
    stats = {
        "tests": 0,
        "questions": 0,
        "skipped_files": 0,
        "explanations_removed": 0,
        "images_removed": 0,
        "copied_indexes": 0,
    }
    dst_dir.mkdir(parents=True, exist_ok=True)

    for src in sorted(src_dir.glob("*.json")):
        dst = dst_dir / src.name

        if src.name.startswith("_"):
            try:
                payload = json.loads(
                    src.read_text(encoding="utf-8", errors="replace")
                )
            except (OSError, json.JSONDecodeError) as exc:
                stats["skipped_files"] += 1
                log(f"  ! {src}: пропущен ({exc})")
                continue
            write_json(dst, payload)
            stats["copied_indexes"] += 1
            continue

        try:
            test = json.loads(src.read_text(encoding="utf-8", errors="replace"))
        except (OSError, json.JSONDecodeError) as exc:
            stats["skipped_files"] += 1
            log(f"  ! {src}: пропущен ({exc})")
            continue
        if not isinstance(test, dict):
            stats["skipped_files"] += 1
            log(f"  ! {src}: пропущен (не объект JSON)")
            continue

        clean, fields, images = sanitize_test(test, keep_explanations)
        write_json(dst, clean)
        stats["tests"] += 1
        stats["questions"] += len(clean.get("questions") or [])
        stats["explanations_removed"] += fields
        stats["images_removed"] += images

    return stats

def copy_referenced_images(
    public_dir: Path, images_dir: Path, questions: list[dict]
) -> dict:
    """Копирует только картинки, на которые ссылаются официальные части заданий."""
    keys: set[str] = set()
    for question in questions:
        keys |= official_image_keys(question)

    copied = 0
    missing = 0
    for key in sorted(keys):
        rel = key.lstrip("/")
        src = images_dir / rel
        if not src.is_file():
            missing += 1
            continue
        dst = public_dir / "images" / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        copied += 1

    return {"images_copied": copied, "images_missing": missing}

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Публичная копия датасета ЗНО/НМТ без чужих разборов."
    )
    parser.add_argument(
        "--src",
        type=Path,
        default=None,
        help="корень датасета (по умолчанию — родитель каталога tools/)",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="каталог публичной копии (по умолчанию <src>/public)",
    )
    parser.add_argument(
        "--subjects",
        default=None,
        help="предметы через запятую/пробел (по умолчанию все)",
    )
    parser.add_argument(
        "--keep-explanations",
        action="store_true",
        help="ничего не вырезать, просто скопировать",
    )
    parser.add_argument(
        "--copy-images",
        action="store_true",
        help="скопировать используемые картинки в public/images/",
    )
    args = parser.parse_args(argv)

    script_root = Path(__file__).resolve().parent.parent
    src = (args.src or script_root).resolve()
    data_dir = src / "data"
    if not data_dir.is_dir():
        parser.error(f"не найден каталог data: {data_dir} (укажите --src)")

    out = (args.out or (src / "public")).resolve()
    data_resolved = data_dir.resolve()
    if out == data_resolved or data_resolved in out.parents:
        parser.error("нельзя писать публичную копию внутрь data/")

    subjects = split_subjects(args.subjects)
    if subjects is None:
        subjects = discover_subjects(data_dir)

    if not subjects:
        parser.error("не найдено ни одного предмета")

    keep = args.keep_explanations
    log(f"Источник: {src}")
    log(f"Куда:     {out}")
    log(f"Режим:    {'копирование как есть' if keep else 'без разборов'}")
    log(f"Предметы: {', '.join(subjects)}")

    out.mkdir(parents=True, exist_ok=True)

    manifest_subjects: dict[str, dict] = {}
    all_public_questions: list[dict] = []

    for subject in subjects:
        log(f"[{subject}]")
        entry = {
            "questions": 0,
            "skipped_lines": 0,
            "tests": 0,
            "questions_in_tests": 0,
            "skipped_test_files": 0,
            "explanations_removed": 0,
            "images_removed": 0,
        }

        bank_src = data_dir / "questions" / f"{subject}.jsonl"
        bank_dst = out / "questions" / f"{subject}.jsonl"
        if bank_src.is_file():
            stats = process_questions_file(bank_src, bank_dst, keep)
            entry["questions"] = stats["questions"]
            entry["skipped_lines"] = stats["skipped_lines"]
            entry["explanations_removed"] += stats["explanations_removed"]
            entry["images_removed"] += stats["images_removed"]
            log(
                f"  заданий: {stats['questions']}"
                f" (пропущено строк: {stats['skipped_lines']},"
                f" удалено картинок: {stats['images_removed']})"
            )
        else:
            log("  нет файла заданий — пропуск")

        tests_src = data_dir / "tests" / subject
        tests_dst = out / "tests" / subject
        if tests_src.is_dir():
            stats = process_tests_dir(tests_src, tests_dst, keep)
            entry["tests"] = stats["tests"]
            entry["questions_in_tests"] = stats["questions"]
            entry["skipped_test_files"] = stats["skipped_files"]
            entry["explanations_removed"] += stats["explanations_removed"]
            entry["images_removed"] += stats["images_removed"]
            log(
                f"  тестов: {stats['tests']}"
                f" (заданий в тестах: {stats['questions']},"
                f" пропущено файлов: {stats['skipped_files']},"
                f" удалено картинок: {stats['images_removed']})"
            )
        else:
            log("  нет каталога тестов — пропуск")

        manifest_subjects[subject] = entry

    totals = {
        "questions": sum(item["questions"] for item in manifest_subjects.values()),
        "tests": sum(item["tests"] for item in manifest_subjects.values()),
        "questions_in_tests": sum(
            item["questions_in_tests"] for item in manifest_subjects.values()
        ),
        "explanations_removed": sum(
            item["explanations_removed"] for item in manifest_subjects.values()
        ),
        "images_removed": sum(
            item["images_removed"] for item in manifest_subjects.values()
        ),
        "skipped_lines": sum(
            item["skipped_lines"] for item in manifest_subjects.values()
        ),
        "skipped_test_files": sum(
            item["skipped_test_files"] for item in manifest_subjects.values()
        ),
    }

    image_stats = {"images_copied": 0, "images_missing": 0}
    if args.copy_images:
        log("[картинки] копирую используемые изображения…")

        for subject in subjects:
            bank = out / "questions" / f"{subject}.jsonl"
            if not bank.is_file():
                continue
            with bank.open("r", encoding="utf-8") as fh:
                for line in fh:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        all_public_questions.append(json.loads(line))
                    except json.JSONDecodeError:
                        continue
        image_stats = copy_referenced_images(
            out, src / "images", all_public_questions
        )
        log(
            f"  скопировано: {image_stats['images_copied']},"
            f" не найдено: {image_stats['images_missing']}"
        )

    manifest = {
        "schema": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "source_dir": src.name,
        "source_site": SOURCE_SITE,
        "keep_explanations": keep,
        "removed_fields": [] if keep else list(EXPLANATION_FIELDS),
        "removed_field_policy": "kept" if keep else "deleted",
        "image_policy": {
            "keep": "картинки официальных частей задания "
            "(question_html, answers, groups, answer_table_html)",
            "drop": "картинки, встречающиеся только в explanation_html",
            "copied_to_public": bool(args.copy_images),
            **image_stats,
        },
        "license_notice": LICENSE_FILE,
        "license_note": "Официальные задания и ответы — материалы УЦОЯО; "
        "разборы «Освіта.ua» исключены; источник — zno.osvita.ua; "
        "датасет собран для личного использования и разработки.",
        "subjects": manifest_subjects,
        "totals": totals,
        "notes": [
            "Официальные задания и ответы — материалы УЦОЯО.",
            "Разборы (explanation_html/explanation_text) — авторский контент "
            "«Освіта.ua» — в публичную копию не входят.",
            "Перед публикацией замените разборы своими или заключите договор "
            "с правообладателем.",
        ],
    }

    write_json(out / MANIFEST_FILE, manifest)
    (out / LICENSE_FILE).write_text(
        build_license_notice(keep), encoding="utf-8", newline="\n"
    )

    log("")
    log(
        f"Готово: заданий {totals['questions']}, тестов {totals['tests']},"
        f" заданий в тестах {totals['questions_in_tests']}"
    )
    log(f"Удалено полей разбора: {totals['explanations_removed']}")
    log(f"Удалено картинок разборов: {totals['images_removed']}")
    log(f"Манифест: {out / MANIFEST_FILE}")
    log(f"Лицензия: {out / LICENSE_FILE}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
