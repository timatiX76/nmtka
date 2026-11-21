mod html;
mod theme;

use eframe::egui;
use eframe::egui::Color32;
use theme::*;
use zno_core::{Dataset, NmtVariant, Question, QuestionType, TestListItem, Topic};

use html::Block;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let root = args
        .get(1)
        .cloned()
        .or_else(|| std::env::var("ZNO_DATA").ok())
        .or_else(find_root)
        .unwrap_or_else(|| ".".to_string());
    let deep_link = match (args.get(2), args.get(3)) {
        (Some(subject), Some(key)) => Some((subject.clone(), key.clone())),
        _ => None,
    };

    let ds = match Dataset::open(&root) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("Не вдалося відкрити датасет {root}:\n{e:#}");
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 860.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("nmtka — ЗНО/НМТ підготовка"),
        ..Default::default()
    };

    eframe::run_native(
        "nmtka — ЗНО/НМТ підготовка",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc, ds, deep_link)))),
    )
}

fn find_root() -> Option<String> {
    let mut starts = Vec::new();
    if let Ok(d) = std::env::current_dir() {
        starts.push(d);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            starts.push(d.to_path_buf());
        }
    }
    starts.push(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(".."),
    );
    for s in starts {
        let mut d = Some(s.as_path());
        for _ in 0..5 {
            if let Some(dir) = d {
                if dir.join("data").join("index.json").is_file() {
                    return Some(dir.to_string_lossy().into_owned());
                }
                d = dir.parent();
            }
        }
    }
    None
}

fn resolve_latex_dir(root: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(v) = std::env::var("ZNO_ASSETS") {
        candidates.push(std::path::PathBuf::from(v));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            candidates.push(d.join("assets").join("latex"));
            candidates.push(d.join("app").join("assets").join("latex"));
            candidates.push(
                d.join("..")
                    .join("app")
                    .join("assets")
                    .join("latex"),
            );
        }
    }
    candidates.push(root.join("app").join("assets").join("latex"));
    candidates.push(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("latex"),
    );
    candidates.into_iter().find(|p| p.is_dir())
}

#[derive(Clone, PartialEq, Eq)]
enum Screen {
    Subjects,
    Hub(String),
    List { subject: String, mode: ListMode },
    Practice,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListMode {
    Nmt,
    Demo,
    Topics,
    Tests,
}

impl ListMode {
    fn title(self) -> &'static str {
        match self {
            ListMode::Nmt => "Тренувальні варіанти НМТ 2027",
            ListMode::Demo => "Пробні варіанти НМТ (демо)",
            ListMode::Topics => "Завдання за темами",
            ListMode::Tests => "Усі тести НМТ/ЗНО",
        }
    }
}

struct Session {
    title: String,
    subject: String,
    questions: Vec<Question>,
    idx: usize,
}

#[derive(Default)]
struct AnswerState {
    selected: Option<usize>,
    multi: Vec<usize>,
    input: String,
    checked: bool,
    correct: bool,
}

struct CurQuestion {
    q_blocks: Vec<Block>,
    e_blocks: Vec<Block>,
}

struct App {
    ds: Dataset,
    screen: Screen,
    session: Option<Session>,
    tests: Vec<TestListItem>,
    nmt: Vec<NmtVariant>,
    topics: Vec<Topic>,
    st: AnswerState,
    cur: Option<CurQuestion>,
    error: Option<String>,
    answered: usize,
    right: usize,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        ds: Dataset,
        deep_link: Option<(String, String)>,
    ) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        theme::load_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx);
        if let Some(dir) = resolve_latex_dir(ds.root()) {
            html::set_latex_dir(dir);
        }

        let mut app = Self {
            ds,
            screen: Screen::Subjects,
            session: None,
            tests: Vec::new(),
            nmt: Vec::new(),
            topics: Vec::new(),
            st: AnswerState::default(),
            cur: None,
            error: None,
            answered: 0,
            right: 0,
        };
        if let Some((subject, key)) = deep_link {
            app.start_test(&subject, &key);
        }
        app
    }

    fn open_subject(&mut self, slug: &str) {
        self.error = None;
        self.tests.clear();
        self.nmt.clear();
        self.topics.clear();
        match self.ds.test_index(slug) {
            Ok(ti) => self.tests = ti.tests,
            Err(e) => self.error = Some(format!("{e:#}")),
        }
        if let Ok(Some(t)) = self.ds.topics(slug) {
            self.topics = t.topics;
        }
        if let Ok(Some(n)) = self.ds.nmt(slug) {
            self.nmt = n.variants;
        }
        self.screen = Screen::Hub(slug.to_string());
    }

    fn start_session(&mut self, subject: &str, title: String, questions: Vec<Question>) {
        self.answered = 0;
        self.right = 0;
        self.session = Some(Session {
            title,
            subject: subject.to_string(),
            questions,
            idx: 0,
        });
        self.screen = Screen::Practice;
        self.prepare();
    }

    fn start_test(&mut self, subject: &str, key: &str) {
        self.error = None;
        match self.ds.load_test(subject, key) {
            Ok(t) => {
                let title = t.title.clone();
                self.start_session(subject, title, t.questions);
            }
            Err(e) => self.error = Some(format!("{e:#}")),
        }
    }

    fn start_topic(&mut self, subject: &str, slug: &str, name: &str) {
        self.error = None;
        match self.ds.load_topic_questions(subject, slug) {
            Ok(qs) => self.start_session(subject, format!("Тема: {name}"), qs),
            Err(e) => self.error = Some(format!("{e:#}")),
        }
    }

    fn start_random(&mut self, subject: &str) {
        self.error = None;
        match self.ds.load_bank_sample(subject, 10) {
            Ok(qs) => self.start_session(subject, "Десятка випадкових завдань".to_string(), qs),
            Err(e) => self.error = Some(format!("{e:#}")),
        }
    }

    fn start_all(&mut self, subject: &str) {
        self.error = None;
        match self.ds.load_questions(subject) {
            Ok(qs) => self.start_session(subject, "Усі завдання з предмета".to_string(), qs),
            Err(e) => self.error = Some(format!("{e:#}")),
        }
    }

    fn prepare(&mut self) {
        self.st = AnswerState::default();
        self.cur = self
            .session
            .as_ref()
            .and_then(|s| s.questions.get(s.idx))
            .map(|q| {
                let mut q_blocks = html::parse(&q.question_html);
                if !q_blocks.iter().any(|b| matches!(b, Block::Image(_))) {
                    for im in &q.images {
                        q_blocks.push(Block::Image(im.path.clone()));
                    }
                }
                let e_blocks = q
                    .explanation_html
                    .as_deref()
                    .map(html::parse)
                    .unwrap_or_default();
                CurQuestion { q_blocks, e_blocks }
            });
    }

    fn check(&mut self) {
        let Some(sess) = self.session.as_ref() else { return };
        let Some(q) = sess.questions.get(sess.idx) else { return };
        let ok = evaluate(q, &self.st);
        self.st.checked = true;
        self.st.correct = ok;
        self.answered += 1;
        if ok {
            self.right += 1;
        }
    }

    fn go_back(&mut self) {
        self.screen = match &self.screen {
            Screen::Subjects => Screen::Subjects,
            Screen::Hub(_) => Screen::Subjects,
            Screen::List { subject, .. } => Screen::Hub(subject.clone()),
            Screen::Practice => match &self.session {
                Some(s) => Screen::Hub(s.subject.clone()),
                None => Screen::Subjects,
            },
        };
    }

    fn subject_name(&self, slug: &str) -> String {
        self.ds
            .subject(slug)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| slug.to_string())
    }

    fn breadcrumb(&self) -> String {
        match &self.screen {
            Screen::Subjects => "Предмети".to_string(),
            Screen::Hub(slug) => self.subject_name(slug),
            Screen::List { subject, mode } => {
                format!("{} · {}", self.subject_name(subject), mode.title())
            }
            Screen::Practice => match &self.session {
                Some(s) => format!("{} · {}", self.subject_name(&s.subject), s.title),
                None => String::new(),
            },
        }
    }

    fn demo_tests(&self) -> Vec<&TestListItem> {
        self.tests
            .iter()
            .filter(|t| t.title.contains("демо") || t.title.contains("пробн"))
            .collect()
    }

    fn ui_subjects(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.label(semi("Оберіть предмет", 28.0).color(TEXT));
        ui.label(
            t(
                format!(
                    "{} тестів · {} завдань",
                    self.ds.index().tests,
                    self.ds.index().questions
                ),
                14.0,
            )
            .color(MUTED),
        );
        ui.add_space(16.0);

        let mut goto: Option<String> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("subjects_grid")
                .num_columns(3)
                .spacing([14.0, 14.0])
                .show(ui, |ui| {
                    for (i, s) in self.ds.subjects().iter().enumerate() {
                        if subject_card(ui, i as u64, s) {
                            goto = Some(s.slug.clone());
                        }
                        if i % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
            ui.add_space(24.0);
        });
        if let Some(slug) = goto {
            self.open_subject(&slug);
        }
    }

    fn ui_hub(&mut self, ui: &mut egui::Ui, slug: &str) {
        let name = self.subject_name(slug);
        let stat = self.ds.subject(slug).cloned();
        let mut action: Option<Screen> = None;

        ui.add_space(6.0);
        ui.label(semi(name, 30.0).color(TEXT));
        if let Some(s) = &stat {
            ui.horizontal(|ui| {
                badge(ui, &format!("{} тестів", s.tests), MUTED, CHIP_BG);
                badge(ui, &format!("{} завдань", s.questions), MUTED, CHIP_BG);
                if s.topics > 0 {
                    badge(ui, &format!("{} тем", s.topics), MUTED, CHIP_BG);
                }
                if s.nmt_variants > 0 {
                    badge(ui, &format!("{} НМТ", s.nmt_variants), ACCENT, ACCENT_SOFT);
                }
            });
        }
        ui.add_space(18.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(900.0);

            if menu_card(
                ui,
                "hub_nmt",
                ph::LIGHTNING,
                "Тренувальні варіанти НМТ 2027",
                &format!("{} варіантів", self.nmt.len()),
            ) {
                action = Some(Screen::List {
                    subject: slug.to_string(),
                    mode: ListMode::Nmt,
                });
            }
            ui.add_space(10.0);
            let demo_count = self.demo_tests().len();
            if menu_card(
                ui,
                "hub_demo",
                ph::PLAY,
                "Пробні варіанти НМТ (демо)",
                &format!("{demo_count} варіантів"),
            ) {
                action = Some(Screen::List {
                    subject: slug.to_string(),
                    mode: ListMode::Demo,
                });
            }
            ui.add_space(10.0);
            if menu_card(
                ui,
                "hub_topics",
                ph::FOLDER_OPEN,
                "Завдання за темами",
                &format!("{} тем", self.topics.len()),
            ) {
                action = Some(Screen::List {
                    subject: slug.to_string(),
                    mode: ListMode::Topics,
                });
            }
            ui.add_space(10.0);
            if menu_card(
                ui,
                "hub_tests",
                ph::ARCHIVE,
                "Усі тести НМТ/ЗНО",
                &format!("{} тестів", self.tests.len()),
            ) {
                action = Some(Screen::List {
                    subject: slug.to_string(),
                    mode: ListMode::Tests,
                });
            }
            ui.add_space(10.0);
            if menu_card(
                ui,
                "hub_random",
                ph::SHUFFLE,
                "Десятка випадкових завдань",
                "10 завдань",
            ) {
                self.start_random(slug);
            }
            ui.add_space(10.0);
            if menu_card(
                ui,
                "hub_all",
                ph::STACK,
                "Усі завдання з предмета",
                &format!(
                    "{} завдань",
                    stat.as_ref().map(|s| s.questions).unwrap_or(0)
                ),
            ) {
                self.start_all(slug);
            }
            ui.add_space(24.0);
        });

        if let Some(screen) = action {
            self.screen = screen;
        }
    }

    fn ui_list(&mut self, ui: &mut egui::Ui, subject: &str, mode: ListMode) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(semi(mode.title(), 24.0).color(TEXT));
        });
        ui.add_space(10.0);

        let mut action: Option<(String, String)> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(1000.0);
            match mode {
                ListMode::Nmt => {
                    for (i, v) in self.nmt.iter().enumerate() {
                        if list_row(ui, ("nmt", i), &v.title, Vec::new(), v.questions_count, ph::LIGHTNING) {
                            action = Some((v.slug.clone(), v.title.clone()));
                        }
                        ui.add_space(8.0);
                    }
                    if self.nmt.is_empty() {
                        empty_state(ui, "Для цього предмета варіантів НМТ немає");
                    }
                }
                ListMode::Demo => {
                    let demos = self.demo_tests();
                    for (i, item) in demos.iter().enumerate() {
                        let badges = test_badges(item);
                        if list_row(ui, ("demo", i), &item.title, badges, item.questions_count, ph::PLAY) {
                            action = Some((item.id.to_string(), item.title.clone()));
                        }
                        ui.add_space(8.0);
                    }
                    if demos.is_empty() {
                        empty_state(ui, "Для цього предмета демоваріантів немає");
                    }
                }
                ListMode::Tests => {
                    for (i, item) in self.tests.iter().enumerate() {
                        let badges = test_badges(item);
                        if list_row(ui, ("tests", i), &item.title, badges, item.questions_count, ph::FILE_TEXT) {
                            action = Some((item.id.to_string(), item.title.clone()));
                        }
                        ui.add_space(8.0);
                    }
                    if self.tests.is_empty() {
                        empty_state(ui, "Для цього предмета тестів немає");
                    }
                }
                ListMode::Topics => {
                    for (i, item) in self.topics.iter().enumerate() {
                        if list_row(ui, ("topics", i), &item.name, Vec::new(), None, ph::NOTE_PENCIL) {
                            action = Some((item.slug.clone(), item.name.clone()));
                        }
                        ui.add_space(8.0);
                    }
                    if self.topics.is_empty() {
                        empty_state(ui, "Для цього предмета тем немає");
                    }
                }
            }
            ui.add_space(24.0);
        });

        if let Some((key, _)) = action {
            match mode {
                ListMode::Topics => {
                    let name = self
                        .topics
                        .iter()
                        .find(|t| t.slug == key)
                        .map(|t| t.name.clone())
                        .unwrap_or_else(|| key.clone());
                    self.start_topic(subject, &key, &name);
                }
                _ => self.start_test(subject, &key),
            }
        }
    }

    fn ui_practice(&mut self, ui: &mut egui::Ui) {
        let Some(sess) = self.session.as_ref() else {
            ui.label(t("Сесія не завантажена", 15.0).color(MUTED));
            return;
        };
        let total = sess.questions.len();
        if total == 0 {
            empty_state(ui, "Немає завдань");
            return;
        }
        let qi = sess.idx.min(total - 1);
        let q = &sess.questions[qi];
        let title = sess.title.clone();

        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label(semi(title, 20.0).color(TEXT));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                score_chip(ui, self.right, self.answered);
            });
        });
        ui.add_space(6.0);

        let frac = (qi + 1) as f32 / total as f32;
        progress_bar(ui, egui::Id::unique("practice_progress"), frac);
        ui.label(t(format!("{} / {}", qi + 1, total), 12.5).color(MUTED));
        ui.add_space(10.0);

        let mut prev = false;
        let mut next = false;
        let mut check = false;

        let root = self.ds.root().to_path_buf();
        let cur = self.cur.as_ref();
        let st = &mut self.st;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    badge(ui, q.kind.label(), ACCENT, ACCENT_SOFT);
                    if let Some(topic) = &q.topic {
                        badge(ui, topic, MUTED, CHIP_BG);
                    }
                    badge(ui, &format!("№{}", q.id), MUTED, CHIP_BG);
                });
                ui.add_space(8.0);

                match cur {
                    Some(c) if !c.q_blocks.is_empty() => html::render(ui, &c.q_blocks, &root),
                    _ => {
                        ui.label(t(&q.text, 16.0).color(TEXT));
                    }
                }

                ui.add_space(14.0);
                answers_ui(ui, q, st);

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(qi > 0, back_button())
                        .clicked()
                    {
                        prev = true;
                    }
                    if !st.checked && primary_button(ui, "Перевірити").clicked() {
                        check = true;
                    }
                    if st.checked && primary_button(ui, "Далі").clicked() {
                        next = true;
                    }
                });

                if st.checked {
                    ui.add_space(14.0);
                    if matches!(q.kind, QuestionType::Matching | QuestionType::Sequence) {
                        result_banner(
                            ui,
                            None,
                            "Самоперевірка: звірте з правильною відповіддю",
                            &correct_answer_text(q),
                        );
                    } else if st.correct {
                        result_banner(ui, Some(true), "Правильно", &correct_answer_text(q));
                    } else {
                        result_banner(ui, Some(false), "Неправильно", &correct_answer_text(q));
                    }
                }

                if st.checked {
                    ui.add_space(14.0);
                    if q.explanation_is_stub() {
                        ui.label(
                            t("Повне пояснення цього завдання доступне лише на сайті", 14.0)
                                .italics()
                                .color(MUTED),
                        );
                    } else if let Some(c) = cur {
                        if !c.e_blocks.is_empty() {
                            egui::CollapsingHeader::new(semi("Пояснення", 17.0).color(ACCENT))
                                .default_open(true)
                                .show(ui, |ui| {
                                    html::render(ui, &c.e_blocks, &root);
                                });
                        }
                    } else if let Some(txt) = &q.explanation_text {
                        ui.label(t(txt, 15.0).color(TEXT));
                    }
                }
                ui.add_space(24.0);
            });

        if check {
            self.check();
        }
        if prev {
            if let Some(s) = self.session.as_mut() {
                s.idx = qi.saturating_sub(1);
            }
            self.prepare();
        }
        if next {
            if let Some(s) = self.session.as_mut() {
                if qi + 1 < total {
                    s.idx = qi + 1;
                }
            }
            self.prepare();
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        paint_bg(ui);

        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if !matches!(self.screen, Screen::Subjects) && ui.add(back_button()).clicked() {
                    self.go_back();
                }
                ui.label(icon(ph::GRADUATION_CAP, 22.0, ACCENT));
                ui.label(semi("nmtka", 19.0).color(ACCENT));
                ui.label(t("· ЗНО/НМТ підготовка", 13.0).color(MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(t(self.breadcrumb(), 13.0).color(MUTED));
                });
            });
            ui.add_space(6.0);
            let r = ui.max_rect();
            ui.painter().line_segment(
                [
                    egui::pos2(r.left(), r.bottom()),
                    egui::pos2(r.right(), r.bottom()),
                ],
                egui::Stroke::new(1.0, Color32::from_rgb(24, 36, 40)),
            );
        });

        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(err) = &self.error {
                egui::Frame::new()
                    .fill(RED_SOFT)
                    .stroke(egui::Stroke::new(1.0, RED.gamma_multiply(0.5)))
                    .corner_radius(10)
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(icon(ph::WARNING_CIRCLE, 18.0, RED));
                            ui.label(t(err, 14.0).color(TEXT));
                        });
                    });
                ui.add_space(8.0);
            }
            match self.screen.clone() {
                Screen::Subjects => self.ui_subjects(ui),
                Screen::Hub(slug) => self.ui_hub(ui, &slug),
                Screen::List { subject, mode } => self.ui_list(ui, &subject, mode),
                Screen::Practice => self.ui_practice(ui),
            }
        });
    }
}

fn paint_bg(ui: &egui::Ui) {
    let rect = ui.max_rect();
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), BG_TOP);
    mesh.colored_vertex(rect.right_top(), BG_TOP);
    mesh.colored_vertex(rect.right_bottom(), BG_BOTTOM);
    mesh.colored_vertex(rect.left_bottom(), BG_BOTTOM);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));

    glow(
        ui,
        egui::pos2(rect.right() - 240.0, rect.top() + 90.0),
        560.0,
        (70, 190, 155, 34),
    );
    glow(
        ui,
        egui::pos2(rect.left() + 120.0, rect.bottom() - 40.0),
        480.0,
        (60, 170, 140, 22),
    );

    let shade_h = 140.0_f32.min(rect.height() * 0.2);
    let mut shade = egui::Mesh::default();
    let y0 = rect.bottom() - shade_h;
    shade.colored_vertex(egui::pos2(rect.left(), y0), Color32::TRANSPARENT);
    shade.colored_vertex(egui::pos2(rect.right(), y0), Color32::TRANSPARENT);
    shade.colored_vertex(
        egui::pos2(rect.right(), rect.bottom()),
        Color32::from_rgba_unmultiplied(0, 0, 0, 42),
    );
    shade.colored_vertex(
        egui::pos2(rect.left(), rect.bottom()),
        Color32::from_rgba_unmultiplied(0, 0, 0, 42),
    );
    shade.add_triangle(0, 1, 2);
    shade.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(shade));
}

fn glow(ui: &egui::Ui, center: egui::Pos2, radius: f32, (r, g, b, a): (u8, u8, u8, u8)) {
    let mut glow = egui::Mesh::default();
    glow.colored_vertex(center, Color32::from_rgba_unmultiplied(r, g, b, a));
    const SEGMENTS: usize = 40;
    for i in 0..=SEGMENTS {
        let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        glow.colored_vertex(
            center + egui::vec2(angle.cos() * radius, angle.sin() * radius),
            Color32::TRANSPARENT,
        );
    }
    for i in 0..SEGMENTS as u32 {
        glow.add_triangle(0, i + 1, i + 2);
    }
    ui.painter().add(egui::Shape::mesh(glow));
}

fn subject_card(ui: &mut egui::Ui, idx: u64, s: &zno_core::SubjectStat) -> bool {
    let size = egui::vec2(300.0, 112.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let k = hover_t(ui.ctx(), ("subject_card", idx), resp.hovered());
    let painter = ui.painter();
    painter.rect(
        rect,
        egui::CornerRadius::same(12),
        mix(CARD_BG, CARD_HOVER, k),
        egui::Stroke::new(1.0 + 0.5 * k, mix(BORDER, ACCENT, k)),
        egui::StrokeKind::Inside,
    );

    let chip = egui::Rect::from_min_size(
        rect.left_top() + egui::vec2(16.0, 16.0),
        egui::vec2(38.0, 38.0),
    );
    painter.rect(
        chip,
        egui::CornerRadius::same(9),
        mix(CHIP_BG, ACCENT_SOFT, k),
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    painter.text(
        chip.center(),
        egui::Align2::CENTER_CENTER,
        subject_icon(&s.slug),
        icon_id(20.0),
        ACCENT,
    );

    let text_x = rect.left() + 68.0;
    let name_g = painter.layout(s.name.clone(), semibold_id(16.5), TEXT, rect.width() - 84.0);
    painter.galley(
        egui::pos2(text_x, rect.top() + 14.0),
        name_g,
        TEXT,
    );

    let line1 = format!("{} тестів · {} завдань", s.tests, s.questions);
    painter.text(
        egui::pos2(text_x, rect.top() + 56.0),
        egui::Align2::LEFT_TOP,
        line1,
        egui::FontId::proportional(12.5),
        MUTED,
    );
    let mut extra = Vec::new();
    if s.topics > 0 {
        extra.push(format!("{} тем", s.topics));
    }
    if s.nmt_variants > 0 {
        extra.push(format!("{} НМТ", s.nmt_variants));
    }
    if !extra.is_empty() {
        painter.text(
            egui::pos2(text_x, rect.top() + 76.0),
            egui::Align2::LEFT_TOP,
            extra.join(" · "),
            egui::FontId::proportional(12.5),
            mix(MUTED, ACCENT, 0.35),
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn menu_card(ui: &mut egui::Ui, id: &str, glyph: &str, title: &str, hint: &str) -> bool {
    let width = ui.available_width().min(900.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 72.0), egui::Sense::click());
    let k = hover_t(ui.ctx(), id, resp.hovered());
    let painter = ui.painter();
    painter.rect(
        rect,
        egui::CornerRadius::same(12),
        mix(CARD_BG, CARD_HOVER, k),
        egui::Stroke::new(1.0 + 0.5 * k, mix(BORDER, ACCENT, k)),
        egui::StrokeKind::Inside,
    );

    let chip = egui::Rect::from_min_size(
        rect.left_center() + egui::vec2(18.0, -19.0),
        egui::vec2(38.0, 38.0),
    );
    painter.rect(
        chip,
        egui::CornerRadius::same(9),
        mix(CHIP_BG, ACCENT_SOFT, k),
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    painter.text(
        chip.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        icon_id(20.0),
        mix(ACCENT, TEXT, 0.3 * k),
    );

    painter.text(
        egui::pos2(rect.left() + 70.0, rect.center().y - 11.0),
        egui::Align2::LEFT_CENTER,
        title,
        semibold_id(17.5),
        TEXT,
    );
    painter.text(
        egui::pos2(rect.left() + 70.0, rect.center().y + 11.0),
        egui::Align2::LEFT_CENTER,
        hint,
        egui::FontId::proportional(13.0),
        MUTED,
    );
    painter.text(
        rect.right_center() - egui::vec2(22.0 - 4.0 * k, 0.0),
        egui::Align2::RIGHT_CENTER,
        ph::ARROW_RIGHT,
        icon_id(18.0),
        mix(MUTED, ACCENT, k),
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn test_badges(t: &TestListItem) -> Vec<(String, Color32, Color32)> {
    let mut out = Vec::new();
    if let Some(y) = t.year {
        out.push((format!("{y}"), ACCENT, ACCENT_SOFT));
    }
    if let Some(k) = &t.kind {
        if !k.is_empty() {
            out.push((k.clone(), MUTED, CHIP_BG));
        }
    }
    if let Some(s) = &t.session {
        if !s.is_empty() {
            out.push((s.clone(), MUTED, CHIP_BG));
        }
    }
    out
}

fn list_row(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    title: &str,
    badges: Vec<(String, Color32, Color32)>,
    count: Option<u32>,
    glyph: &str,
) -> bool {
    let width = ui.available_width().min(1000.0);
    let height = if badges.is_empty() { 56.0 } else { 66.0 };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let k = hover_t(ui.ctx(), id, resp.hovered());
    let painter = ui.painter();
    painter.rect(
        rect,
        egui::CornerRadius::same(12),
        mix(CARD_BG, CARD_HOVER, k),
        egui::Stroke::new(1.0 + 0.5 * k, mix(BORDER, ACCENT, k)),
        egui::StrokeKind::Inside,
    );

    let chip = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 34.0, rect.center().y),
        egui::vec2(32.0, 32.0),
    );
    painter.rect(
        chip,
        egui::CornerRadius::same(8),
        mix(CHIP_BG, ACCENT_SOFT, k),
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    painter.text(
        chip.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        icon_id(16.0),
        mix(ACCENT, TEXT, 0.3 * k),
    );

    let text_x = rect.left() + 62.0;
    let title_y = if badges.is_empty() {
        rect.center().y
    } else {
        rect.center().y - 10.0
    };
    let max_tw = rect.right() - text_x - 150.0;
    let g = painter.layout(title.to_string(), semibold_id(16.0), TEXT, max_tw.max(200.0));
    painter.galley(egui::pos2(text_x, title_y - g.size().y / 2.0), g, TEXT);

    let mut badge_x = text_x;
    if !badges.is_empty() {
        let by = egui::pos2(badge_x, rect.center().y + 12.0);
        for (text, fg, bg) in &badges {
            badge_x = paint_pill(painter, egui::pos2(badge_x, by.y), text, *fg, *bg, false) + 6.0;
        }
    }

    if let Some(n) = count {
        let text = format!("{n} завдань");
        let g = painter.layout(
            text.clone(),
            egui::FontId::proportional(12.0),
            MUTED,
            f32::INFINITY,
        );
        let pw = g.size().x + 16.0;
        paint_pill(
            painter,
            egui::pos2(rect.right() - 44.0 - pw, rect.center().y),
            &text,
            mix(MUTED, TEXT, 0.3 * k),
            CHIP_BG,
            false,
        );
    }
    painter.text(
        egui::pos2(rect.right() - 22.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        ph::CARET_RIGHT,
        icon_id(16.0),
        mix(MUTED, ACCENT, k),
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn paint_pill(
    painter: &egui::Painter,
    anchor: egui::Pos2,
    text: &str,
    fg: Color32,
    bg: Color32,
    _right_align: bool,
) -> f32 {
    let g = painter.layout(text.to_string(), egui::FontId::proportional(12.0), fg, f32::INFINITY);
    let w = g.size().x + 16.0;
    let h = 20.0;
    let rect = egui::Rect::from_min_size(
        egui::pos2(anchor.x, anchor.y - h / 2.0),
        egui::vec2(w, h),
    );
    painter.rect(
        rect,
        egui::CornerRadius::same(7),
        bg,
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(12.0), fg);
    rect.right()
}

fn progress_bar(ui: &mut egui::Ui, id: egui::Id, target: f32) {
    let width = ui.available_width();
    let h = 10.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, h), egui::Sense::hover());
    let frac = ui
        .ctx()
        .animate_value_with_time(id, target.clamp(0.0, 1.0), 0.45);
    let painter = ui.painter();
    painter.rect(
        rect,
        egui::CornerRadius::same(5),
        FAINT_BG,
        egui::Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    if frac > 0.003 {
        let fill_w = (rect.width() * frac).max(h);
        let fill = egui::Rect::from_min_size(rect.left_top(), egui::vec2(fill_w, h));
        painter.rect(
            fill,
            egui::CornerRadius::same(5),
            ACCENT,
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    }
}

fn score_chip(ui: &mut egui::Ui, right: usize, answered: usize) {
    chip_frame(ACCENT_SOFT, None).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(icon(ph::LIST_CHECKS, 15.0, ACCENT));
            ui.label(semi(format!("{right} / {answered}"), 15.0).color(ACCENT));
        });
    });
}

fn result_banner(ui: &mut egui::Ui, ok: Option<bool>, headline: &str, answer: &str) {
    let (bg, border, fg, glyph) = match ok {
        Some(true) => (GREEN_SOFT, GREEN, GREEN, ph::SEAL_CHECK),
        Some(false) => (RED_SOFT, RED, RED, ph::X_CIRCLE),
        None => (CHIP_BG, BORDER, MUTED, ph::QUESTION),
    };
    egui::Frame::new()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, border.gamma_multiply(0.55)))
        .corner_radius(12)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(icon(glyph, 20.0, fg));
                ui.label(semi(headline, 17.0).color(fg));
            });
            if !answer.is_empty() {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Правильна відповідь: ", 15.0).color(MUTED));
                    ui.label(semi(answer, 15.0).color(TEXT));
                });
            }
        });
}

fn back_button() -> egui::Button<'static> {
    egui::Button::new(semi("← Назад", 14.0).color(TEXT))
        .corner_radius(10)
        .fill(CHIP_BG)
}

fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let txt = egui::RichText::new(format!("{label} →"))
        .font(egui::FontId::new(15.0, theme::semi_family()))
        .color(Color32::from_rgb(9, 18, 15));
    ui.add(egui::Button::new(txt).fill(ACCENT).corner_radius(10))
}

fn empty_state(ui: &mut egui::Ui, text: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        ui.label(icon(ph::TRAY, 44.0, mix(MUTED, CARD_BG, 0.45)));
        ui.add_space(8.0);
        ui.label(t(text, 15.0).color(MUTED));
    });
}

fn semibold_id(size: f32) -> egui::FontId {
    egui::FontId::new(size, theme::semi_family())
}

fn norm(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .replace(',', ".")
        .replace(' ', "")
        .replace('\u{2212}', "-")
}

fn evaluate(q: &Question, st: &AnswerState) -> bool {
    match q.kind {
        QuestionType::SingleChoice => st.selected.is_some() && st.selected == q.correct_index(),
        QuestionType::MultiChoice => {
            let mut a = st.multi.clone();
            a.sort_unstable();
            a.dedup();
            let mut b = q.correct_indices();
            b.sort_unstable();
            !a.is_empty() && a == b
        }
        QuestionType::ShortAnswer => match q.correct_text() {
            Some(exp) => norm(&st.input) == norm(exp),
            None => false,
        },
        _ => true,
    }
}

fn marker_for(q: &Question, letter: char) -> String {
    let idx = (letter as u8).wrapping_sub(b'a') as usize;
    q.answers
        .get(idx)
        .map(|a| a.marker.clone())
        .unwrap_or_else(|| letter.to_string())
}

fn correct_answer_text(q: &Question) -> String {
    match q.kind {
        QuestionType::SingleChoice => q
            .correct_marker()
            .map(|m| m.to_string())
            .or_else(|| q.correct_text().map(|s| s.to_string()))
            .unwrap_or_default(),
        QuestionType::MultiChoice => q
            .correct_indices()
            .iter()
            .filter_map(|&i| q.answers.get(i).map(|a| a.marker.clone()))
            .collect::<Vec<_>>()
            .join(", "),
        QuestionType::ShortAnswer => q.correct_text().unwrap_or_default().to_string(),
        QuestionType::Matching => q
            .correct_pairs()
            .iter()
            .map(|(n, l)| format!("{n} → {}", marker_for(q, *l)))
            .collect::<Vec<_>>()
            .join(", "),
        _ => q.correct.join(", "),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum RowState {
    Idle,
    Selected,
    Correct,
    Wrong,
    Dim,
}

fn answer_row(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    marker: &str,
    text: &str,
    state: RowState,
    enabled: bool,
    boxed: bool,
    box_on: bool,
) -> bool {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let width = ui.available_width();

    let probe = ui.painter().clone();
    let g = probe.layout(text.to_string(), egui::FontId::proportional(16.0), TEXT, width - 118.0);
    let height = (g.size().y + 26.0).max(if boxed { 50.0 } else { 52.0 });

    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), sense);
    let k = hover_t(ui.ctx(), id, resp.hovered() && enabled);
    let painter = ui.painter();

    let (fill, border, marker_bg, marker_fg) = match state {
        RowState::Idle => (
            mix(CARD_BG, CARD_HOVER, k),
            mix(BORDER, Color32::from_rgb(64, 96, 90), k),
            CHIP_BG,
            ACCENT,
        ),
        RowState::Selected => (
            mix(CARD_HOVER, CHIP_ACTIVE, 0.4 + 0.2 * k),
            ACCENT,
            ACCENT_SOFT,
            ACCENT,
        ),
        RowState::Correct => (GREEN_SOFT, GREEN, GREEN_SOFT, GREEN),
        RowState::Wrong => (RED_SOFT, RED, RED_SOFT, RED),
        RowState::Dim => (CARD_BG, BORDER, CHIP_BG, mix(MUTED, CARD_BG, 0.5)),
    };
    painter.rect(
        rect,
        egui::CornerRadius::same(12),
        fill,
        egui::Stroke::new(if state == RowState::Selected { 1.5 } else { 1.0 }, border),
        egui::StrokeKind::Inside,
    );

    let mx = rect.left() + 14.0;
    let mrect = egui::Rect::from_center_size(
        egui::pos2(mx + 15.0, rect.center().y),
        egui::vec2(30.0, 30.0),
    );
    if boxed {
        painter.rect(
            mrect.shrink(5.0),
            egui::CornerRadius::same(6),
            if box_on { ACCENT } else { Color32::TRANSPARENT },
            egui::Stroke::new(1.5, if box_on { ACCENT } else { marker_fg.gamma_multiply(0.8) }),
            egui::StrokeKind::Inside,
        );
        if box_on {
            painter.text(
                mrect.center(),
                egui::Align2::CENTER_CENTER,
                ph::CHECK,
                icon_id(14.0),
                Color32::from_rgb(9, 18, 15),
            );
        }
    } else {
        painter.rect(
            mrect,
            egui::CornerRadius::same(8),
            marker_bg,
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
        painter.text(
            mrect.center(),
            egui::Align2::CENTER_CENTER,
            marker,
            egui::FontId::new(14.0, theme::semi_family()),
            marker_fg,
        );
    }

    let text_color = match state {
        RowState::Correct => GREEN,
        RowState::Wrong => RED,
        RowState::Dim => mix(TEXT, CARD_BG, 0.45),
        _ => TEXT,
    };
    let g = painter.layout(
        text.to_string(),
        egui::FontId::proportional(16.0),
        text_color,
        width - 118.0,
    );
    painter.galley(
        egui::pos2(rect.left() + 58.0, rect.center().y - g.size().y / 2.0),
        g,
        text_color,
    );

    match state {
        RowState::Correct => {
            painter.text(
                egui::pos2(rect.right() - 18.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                ph::CHECK,
                icon_id(17.0),
                GREEN,
            );
        }
        RowState::Wrong => {
            painter.text(
                egui::pos2(rect.right() - 18.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                ph::X,
                icon_id(17.0),
                RED,
            );
        }
        _ => {}
    }

    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    } else {
        false
    }
}

fn answers_ui(ui: &mut egui::Ui, q: &Question, st: &mut AnswerState) {
    let checked = st.checked;
    match q.kind {
        QuestionType::SingleChoice => {
            let correct = q.correct_index();
            ui.add_space(2.0);
            for (i, a) in q.answers.iter().enumerate() {
                let state = if checked {
                    if correct == Some(i) {
                        RowState::Correct
                    } else if st.selected == Some(i) {
                        RowState::Wrong
                    } else {
                        RowState::Dim
                    }
                } else if st.selected == Some(i) {
                    RowState::Selected
                } else {
                    RowState::Idle
                };
                let id = ui.scope_id().with(("answer", q.id, i));
                if answer_row(ui, id, &a.marker, &html::plain_text(&a.html), state, !checked, false, false) {
                    st.selected = Some(i);
                }
                ui.add_space(6.0);
            }
        }
        QuestionType::MultiChoice => {
            let correct = q.correct_indices();
            ui.add_space(2.0);
            for (i, a) in q.answers.iter().enumerate() {
                let on = st.multi.contains(&i);
                let state = if checked {
                    if correct.contains(&i) {
                        RowState::Correct
                    } else if on {
                        RowState::Wrong
                    } else {
                        RowState::Dim
                    }
                } else if on {
                    RowState::Selected
                } else {
                    RowState::Idle
                };
                let id = ui.scope_id().with(("answer", q.id, i));
                if answer_row(ui, id, &a.marker, &html::plain_text(&a.html), state, !checked, true, on) {
                    if !st.multi.contains(&i) {
                        st.multi.push(i);
                    }
                }
                ui.add_space(6.0);
            }
        }
        QuestionType::ShortAnswer => {
            ui.add_space(2.0);
            ui.add(
                egui::TextEdit::singleline(&mut st.input)
                    .hint_text("Введіть відповідь")
                    .desired_width((ui.available_width().min(420.0)).max(240.0)),
            );
            ui.add_space(4.0);
            ui.label(
                t("Дробову частину можна писати через точку або кому", 13.0).color(MUTED),
            );
        }
        _ => {
            if let Some(groups) = &q.groups {
                ui.add_space(2.0);
                for g in groups {
                    egui::Frame::new()
                        .fill(CARD_BG)
                        .stroke(egui::Stroke::new(1.0, BORDER))
                        .corner_radius(12)
                        .inner_margin(egui::Margin::same(14))
                        .show(ui, |ui| {
                            if let Some(title) = &g.title {
                                ui.label(semi(title, 14.5).color(MUTED));
                                ui.add_space(4.0);
                            }
                            for a in &g.answers {
                                ui.horizontal(|ui| {
                                    ui.label(t(&a.marker, 15.0).color(ACCENT));
                                    ui.label(t(html::plain_text(&a.html), 15.0).color(TEXT));
                                });
                            }
                        });
                    ui.add_space(8.0);
                }
            }
            ui.label(semi("Варіанти:", 15.0).color(MUTED));
            ui.add_space(4.0);
            for a in &q.answers {
                egui::Frame::new()
                    .fill(CARD_BG)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .corner_radius(12)
                    .inner_margin(egui::Margin::symmetric(14, 9))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(t(&a.marker, 15.0).color(ACCENT));
                            ui.label(t(html::plain_text(&a.html), 15.0).color(TEXT));
                        });
                    });
                ui.add_space(6.0);
            }
        }
    }
}

