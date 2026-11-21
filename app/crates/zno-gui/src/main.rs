mod html;

use eframe::egui;
use zno_core::{Dataset, NmtVariant, Question, QuestionType, TestListItem, Topic};

use html::Block;

const BG_TOP: egui::Color32 = egui::Color32::from_rgb(8, 12, 16);
const BG_BOTTOM: egui::Color32 = egui::Color32::from_rgb(13, 45, 40);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(17, 25, 29);
const CARD_HOVER: egui::Color32 = egui::Color32::from_rgb(25, 40, 44);
const BORDER: egui::Color32 = egui::Color32::from_rgb(42, 74, 68);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(88, 205, 168);
const TEXT: egui::Color32 = egui::Color32::from_rgb(226, 236, 238);
const MUTED: egui::Color32 = egui::Color32::from_rgb(138, 158, 162);
const GREEN: egui::Color32 = egui::Color32::from_rgb(96, 214, 150);
const RED: egui::Color32 = egui::Color32::from_rgb(238, 112, 112);

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
            eprintln!("Не удалось открыть датасет {root}:\n{e:#}");
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 860.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("ЗНО/НМТ — підготовка"),
        ..Default::default()
    };

    eframe::run_native(
        "ЗНО/НМТ — підготовка",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc, ds, deep_link)))),
    )
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
        if let Some(dir) = resolve_latex_dir(ds.root()) {
            html::set_latex_dir(dir);
        }

        use egui::{FontFamily, FontId, TextStyle};
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(TEXT);
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        visuals.window_fill = CARD_BG;
        visuals.extreme_bg_color = egui::Color32::from_rgb(10, 15, 19);
        visuals.faint_bg_color = egui::Color32::from_rgb(20, 30, 34);
        visuals.hyperlink_color = ACCENT;
        visuals.selection.bg_fill = egui::Color32::from_rgb(26, 92, 78);
        visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(22, 33, 37);
        visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(22, 33, 37);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(30, 50, 54);
        visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(30, 50, 54);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(34, 62, 62);
        cc.egui_ctx.set_visuals(visuals);

        cc.egui_ctx.all_styles_mut(|style| {
            style.text_styles = [
                (TextStyle::Heading, FontId::new(26.0, FontFamily::Proportional)),
                (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
                (TextStyle::Button, FontId::new(16.0, FontFamily::Proportional)),
                (TextStyle::Small, FontId::new(13.0, FontFamily::Proportional)),
                (TextStyle::Monospace, FontId::new(15.0, FontFamily::Monospace)),
            ]
            .into();
            style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        });

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

    fn subject_name(&self, slug: &str) -> String {        self.ds
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
        ui.heading(egui::RichText::new("Оберіть предмет").color(TEXT));
        ui.label(
            egui::RichText::new(format!(
                "{} тестів · {} завдань · {} формул",
                self.ds.index().tests,
                self.ds.index().questions,
                10214
            ))
            .color(MUTED),
        );
        ui.add_space(14.0);

        let mut goto: Option<String> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("subjects_grid")
                .num_columns(3)
                .spacing([12.0, 12.0])
                .show(ui, |ui| {
                    for (i, s) in self.ds.subjects().iter().enumerate() {
                        egui::Frame::new()
                            .inner_margin(14)
                            .fill(CARD_BG)
                            .stroke(egui::Stroke::new(1.0, BORDER))
                            .corner_radius(10)
                            .show(ui, |ui| {
                                ui.set_width(300.0);
                                ui.vertical(|ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new(&s.name)
                                                    .size(17.0)
                                                    .strong()
                                                    .color(TEXT),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        goto = Some(s.slug.clone());
                                    }
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} тестів · {} завдань",
                                            s.tests, s.questions
                                        ))
                                        .small()
                                        .color(MUTED),
                                    );
                                    let mut extra = Vec::new();
                                    if s.topics > 0 {
                                        extra.push(format!("{} тем", s.topics));
                                    }
                                    if s.nmt_variants > 0 {
                                        extra.push(format!("{} НМТ", s.nmt_variants));
                                    }
                                    if !extra.is_empty() {
                                        ui.label(
                                            egui::RichText::new(extra.join(" · "))
                                                .small()
                                                .color(MUTED),
                                        );
                                    }
                                });
                            });
                        if i % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
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
        ui.heading(egui::RichText::new(name).color(TEXT).size(30.0));
        if let Some(s) = &stat {
            ui.label(
                egui::RichText::new(format!(
                    "{} тестів · {} завдань · {} тем",
                    s.tests, s.questions, s.topics
                ))
                .color(MUTED),
            );
        }
        ui.add_space(18.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(900.0);

            if menu_card(
                ui,
                "Тренувальні варіанти НМТ 2027",
                &format!("{} варіантів", self.nmt.len()),
            ) {
                action = Some(Screen::List {
                    subject: slug.to_string(),
                    mode: ListMode::Nmt,
                });
            }
            ui.add_space(8.0);
            let demo_count = self.demo_tests().len();
            if menu_card(
                ui,
                "Пробні варіанти НМТ (демо)",
                &format!("{demo_count} варіантів"),
            ) {
                action = Some(Screen::List {
                    subject: slug.to_string(),
                    mode: ListMode::Demo,
                });
            }
            ui.add_space(8.0);
            if menu_card(
                ui,
                "Завдання за темами",
                &format!("{} тем", self.topics.len()),
            ) {
                action = Some(Screen::List {
                    subject: slug.to_string(),
                    mode: ListMode::Topics,
                });
            }
            ui.add_space(8.0);
            if menu_card(
                ui,
                "Усі тести НМТ/ЗНО",
                &format!("{} тестів", self.tests.len()),
            ) {
                action = Some(Screen::List {
                    subject: slug.to_string(),
                    mode: ListMode::Tests,
                });
            }
            ui.add_space(8.0);
            if menu_card(ui, "Десятка випадкових завдань", "10 завдань") {
                self.start_random(slug);
            }
            ui.add_space(8.0);
            if menu_card(
                ui,
                "Усі завдання з предмета",
                &format!(
                    "{} завдань",
                    stat.as_ref().map(|s| s.questions).unwrap_or(0)
                ),
            ) {
                self.start_all(slug);
            }
            ui.add_space(20.0);
        });

        if let Some(screen) = action {
            self.screen = screen;
        }
    }

    fn ui_list(&mut self, ui: &mut egui::Ui, subject: &str, mode: ListMode) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("← Назад").clicked() {
                self.screen = Screen::Hub(subject.to_string());
            }
            ui.heading(egui::RichText::new(mode.title()).color(TEXT));
        });
        ui.add_space(8.0);

        let mut action: Option<(String, String)> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_max_width(1000.0);
            match mode {
                ListMode::Nmt => {
                    for v in &self.nmt {
                        let label = format!(
                            "{}   ·   {} завдань",
                            v.title,
                            v.questions_count.unwrap_or(0)
                        );
                        if ui.selectable_label(false, label).clicked() {
                            action = Some((v.slug.clone(), v.title.clone()));
                        }
                    }
                    if self.nmt.is_empty() {
                        ui.label(egui::RichText::new("Для цього предмета варіантів НМТ немає").color(MUTED));
                    }
                }
                ListMode::Demo => {
                    let demos = self.demo_tests();
                    for t in &demos {
                        let label = format!(
                            "{}   ·   {} завдань",
                            t.title,
                            t.questions_count.unwrap_or(0)
                        );
                        if ui.selectable_label(false, label).clicked() {
                            action = Some((t.id.to_string(), t.title.clone()));
                        }
                    }
                    if demos.is_empty() {
                        ui.label(egui::RichText::new("Для цього предмета демоваріантів немає").color(MUTED));
                    }
                }
                ListMode::Tests => {
                    for t in &self.tests {
                        let label = format!(
                            "{}   ·   {} завдань",
                            t.title,
                            t.questions_count.unwrap_or(0)
                        );
                        if ui.selectable_label(false, label).clicked() {
                            action = Some((t.id.to_string(), t.title.clone()));
                        }
                    }
                }
                ListMode::Topics => {
                    for t in &self.topics {
                        if ui.selectable_label(false, &t.name).clicked() {
                            action = Some((t.slug.clone(), t.name.clone()));
                        }
                    }
                    if self.topics.is_empty() {
                        ui.label(egui::RichText::new("Для цього предмета тем немає").color(MUTED));
                    }
                }
            }
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
            ui.label("Сессия не загружена");
            return;
        };
        let total = sess.questions.len();
        if total == 0 {
            ui.label("Нет заданий");
            return;
        }
        let qi = sess.idx.min(total - 1);
        let q = &sess.questions[qi];
        let title = sess.title.clone();

        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new(title).color(TEXT).size(22.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("✔ {} / {}", self.right, self.answered))
                        .color(ACCENT)
                        .strong(),
                );
            });
        });
        ui.add_space(4.0);

        let frac = (qi + 1) as f32 / total as f32;
        let bar = egui::ProgressBar::new(frac)
            .desired_width(ui.available_width())
            .text(egui::RichText::new(format!("{} / {}", qi + 1, total)).small());
        ui.add(bar);
        ui.add_space(8.0);

        let mut prev = false;
        let mut next = false;
        let mut check = false;

        let root = self.ds.root().to_path_buf();
        let cur = self.cur.as_ref();
        let st = &mut self.st;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(q.kind.label())
                            .small()
                            .strong()
                            .color(ACCENT),
                    );
                    if let Some(t) = &q.topic {
                        ui.label(egui::RichText::new(format!("· {t}")).small().color(MUTED));
                    }
                    ui.label(
                        egui::RichText::new(format!("· №{}", q.id))
                            .small()
                            .color(MUTED),
                    );
                });
                ui.add_space(6.0);

                match cur {
                    Some(c) if !c.q_blocks.is_empty() => html::render(ui, &c.q_blocks, &root),
                    _ => {
                        ui.label(&q.text);
                    }
                }

                ui.add_space(12.0);
                answers_ui(ui, q, st);

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(qi > 0, egui::Button::new("← Назад"))
                        .clicked()
                    {
                        prev = true;
                    }
                    if !st.checked && ui.button("Перевірити").clicked() {
                        check = true;
                    }
                    if st.checked && ui.button("Далі →").clicked() {
                        next = true;
                    }
                });

                if st.checked {
                    ui.add_space(10.0);
                    if matches!(q.kind, QuestionType::Matching | QuestionType::Sequence) {
                        ui.label(
                            egui::RichText::new("Самоперевірка: звірте з правильною відповіддю")
                                .italics()
                                .color(MUTED),
                        );
                    } else if st.correct {
                        ui.label(egui::RichText::new("✔ Правильно").color(GREEN).strong());
                    } else {
                        ui.label(egui::RichText::new("✘ Неправильно").color(RED).strong());
                    }
                    let ans = correct_answer_text(q);
                    if !ans.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("Правильна відповідь: {ans}"))
                                .color(TEXT)
                                .strong(),
                        );
                    }
                }

                if st.checked {
                    ui.add_space(10.0);
                    if q.explanation_is_stub() {
                        ui.label(
                            egui::RichText::new(
                                "Повне пояснення цього завдання доступне лише на сайті",
                            )
                            .italics()
                            .color(MUTED),
                        );
                    } else if let Some(c) = cur {
                        if !c.e_blocks.is_empty() {
                            egui::CollapsingHeader::new("Пояснення")
                                .default_open(true)
                                .show(ui, |ui| {
                                    html::render(ui, &c.e_blocks, &root);
                                });
                        }
                    } else if let Some(txt) = &q.explanation_text {
                        ui.label(txt);
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
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if !matches!(self.screen, Screen::Subjects) && ui.button("← Назад").clicked() {
                    self.go_back();
                }
                ui.label(
                    egui::RichText::new("ЗНО / НМТ")
                        .strong()
                        .size(18.0)
                        .color(ACCENT),
                );
                ui.label(egui::RichText::new("підготовка").small().color(MUTED));
                ui.separator();
                ui.label(egui::RichText::new(self.breadcrumb()).color(MUTED));
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(err) = &self.error {
                ui.colored_label(RED, err);
                ui.separator();
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

    let center = egui::pos2(rect.right() - 240.0, rect.top() + 90.0);
    let radius = 560.0_f32;
    let mut glow = egui::Mesh::default();
    glow.colored_vertex(
        center,
        egui::Color32::from_rgba_unmultiplied(70, 190, 155, 34),
    );
    const SEGMENTS: usize = 40;
    for i in 0..=SEGMENTS {
        let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        glow.colored_vertex(
            center + egui::vec2(a.cos() * radius, a.sin() * radius),
            egui::Color32::TRANSPARENT,
        );
    }
    for i in 0..SEGMENTS as u32 {
        glow.add_triangle(0, i + 1, i + 2);
    }
    ui.painter().add(egui::Shape::mesh(glow));
}

fn menu_card(ui: &mut egui::Ui, title: &str, hint: &str) -> bool {
    let width = ui.available_width().min(900.0);
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(width, 68.0), egui::Sense::click());
    let hovered = resp.hovered();
    let painter = ui.painter();
    painter.rect(
        rect,
        egui::CornerRadius::same(10),
        if hovered { CARD_HOVER } else { CARD_BG },
        egui::Stroke::new(1.0, if hovered { ACCENT } else { BORDER }),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.left_center() + egui::vec2(22.0, -9.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(19.0),
        TEXT,
    );
    painter.text(
        rect.left_center() + egui::vec2(22.0, 14.0),
        egui::Align2::LEFT_CENTER,
        hint,
        egui::FontId::proportional(13.0),
        MUTED,
    );
    painter.text(
        rect.right_center() - egui::vec2(22.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        "Перейти →",
        egui::FontId::proportional(15.0),
        if hovered { TEXT } else { ACCENT },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
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

fn answers_ui(ui: &mut egui::Ui, q: &Question, st: &mut AnswerState) {
    let checked = st.checked;
    match q.kind {
        QuestionType::SingleChoice => {
            let correct = q.correct_index();
            for (i, a) in q.answers.iter().enumerate() {
                let mut rt = egui::RichText::new(format!(
                    "{}   {}",
                    a.marker,
                    html::plain_text(&a.html)
                ))
                .size(16.0);
                if checked {
                    if correct == Some(i) {
                        rt = rt.color(GREEN).strong();
                    } else if st.selected == Some(i) {
                        rt = rt.color(RED);
                    }
                }
                if ui.selectable_label(st.selected == Some(i), rt).clicked() && !checked {
                    st.selected = Some(i);
                }
            }
        }
        QuestionType::MultiChoice => {
            let correct = q.correct_indices();
            for (i, a) in q.answers.iter().enumerate() {
                let mut on = st.multi.contains(&i);
                let mut rt = egui::RichText::new(format!(
                    "{}   {}",
                    a.marker,
                    html::plain_text(&a.html)
                ))
                .size(16.0);
                if checked {
                    if correct.contains(&i) {
                        rt = rt.color(GREEN).strong();
                    } else if on {
                        rt = rt.color(RED);
                    }
                }
                let resp = ui.add_enabled(!checked, egui::Checkbox::new(&mut on, rt));
                if resp.changed() {
                    if on {
                        if !st.multi.contains(&i) {
                            st.multi.push(i);
                        }
                    } else {
                        st.multi.retain(|x| *x != i);
                    }
                }
            }
        }
        QuestionType::ShortAnswer => {
            ui.add_enabled(
                !checked,
                egui::TextEdit::singleline(&mut st.input)
                    .hint_text("введіть відповідь")
                    .desired_width(300.0),
            );
            ui.label(
                egui::RichText::new("Дробову частину можна писати через точку або кому")
                    .small()
                    .color(MUTED),
            );
        }
        _ => {
            if let Some(groups) = &q.groups {
                for g in groups {
                    if let Some(title) = &g.title {
                        ui.label(egui::RichText::new(title).strong().color(TEXT));
                    }
                    for a in &g.answers {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}   {}",
                                a.marker,
                                html::plain_text(&a.html)
                            ))
                            .color(TEXT),
                        );
                    }
                    ui.add_space(4.0);
                }
            }
            ui.label(egui::RichText::new("Варіанти:").strong().color(TEXT));
            for a in &q.answers {
                ui.label(
                    egui::RichText::new(format!(
                        "{}   {}",
                        a.marker,
                        html::plain_text(&a.html)
                    ))
                    .color(TEXT),
                );
            }
        }
    }
}
