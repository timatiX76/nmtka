use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use eframe::egui;

static LATEX_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_latex_dir(p: PathBuf) {
    let _ = LATEX_DIR.set(p);
}

fn latex_dir() -> Option<&'static Path> {
    LATEX_DIR.get().map(|p| p.as_path()).filter(|p| p.is_dir())
}

fn sha1_hex(data: &[u8]) -> String {
    let mut h: [u32; 5] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
    let ml = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&ml.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    h.iter().map(|v| format!("{v:08x}")).collect()
}

#[derive(Clone, Debug, Default)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub sup: bool,
    pub sub: bool,
    pub latex: bool,
}

#[derive(Clone, Debug)]
pub enum Block {
    Para(Vec<Span>),
    Image(String),
    Table(Vec<Vec<String>>),
}

#[derive(Default, Clone, Copy)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
    sup: bool,
    sub: bool,
    latex: bool,
}

impl Style {
    fn span(&self, text: String) -> Span {
        Span {
            text,
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            sup: self.sup,
            sub: self.sub,
            latex: self.latex,
        }
    }
}

const BLOCK_TAGS: &[&str] = &[
    "p", "div", "li", "tr", "table", "ul", "ol", "h1", "h2", "h3", "h4", "blockquote", "center",
    "thead", "tbody", "caption", "hr", "br",
];

const SKIP_TAGS: &[&str] = &["script", "style", "audio", "iframe", "video", "svg", "head"];

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos..];
        let semi = after.find(';').filter(|&e| e <= 12);
        let Some(semi) = semi else {
            out.push('&');
            rest = &after[1..];
            continue;
        };
        let ent = &after[1..semi];
        let decoded = match ent {
            "nbsp" => Some(' '),
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "ndash" => Some('–'),
            "mdash" => Some('—'),
            "laquo" => Some('«'),
            "raquo" => Some('»'),
            "deg" => Some('°'),
            _ => {
                if let Some(hex) = ent.strip_prefix("#x").or_else(|| ent.strip_prefix("#X")) {
                    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                } else if let Some(dec) = ent.strip_prefix('#') {
                    dec.parse::<u32>().ok().and_then(char::from_u32)
                } else {
                    None
                }
            }
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &after[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn tag_end(html: &str, from: usize) -> Option<usize> {
    html[from..].find('>').map(|p| from + p)
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let lower = tag.to_ascii_lowercase();
    let pat = format!("{name}=");
    let idx = 0;
    while let Some(pos) = lower[idx..].find(&pat) {
        let start = idx + pos + pat.len();
        let rest = &tag[start..];
        let mut chars = rest.chars();
        match chars.next()? {
            '"' | '\'' => {
                let q = rest.chars().next()?;
                let inner = &rest[1..];
                return inner.find(q).map(|e| &inner[..e]);
            }
            _ => {
                let end = rest
                    .find(|c: char| c.is_whitespace() || c == '>')
                    .unwrap_or(rest.len());
                return Some(&rest[..end]);
            }
        }
    }
    None
}

fn skip_to_close(html: &str, from: usize, tag: &str) -> usize {
    let needle = format!("</{tag}");
    let lower = html[from..].to_ascii_lowercase();
    match lower.find(&needle) {
        Some(p) => {
            let close = from + p;
            tag_end(html, close).map(|e| e + 1).unwrap_or(html.len())
        }
        None => html.len(),
    }
}

fn find_table_end(html: &str, from: usize) -> Option<usize> {
    let lower = html[from..].to_ascii_lowercase();
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < lower.len() {
        if let Some(p) = lower[i..].find('<') {
            let abs = i + p;
            let rest = &lower[abs..];
            if rest.starts_with("<table") {
                depth += 1;
            } else if rest.starts_with("</table") {
                depth -= 1;
                if depth == 0 {
                    return tag_end(html, abs).map(|e| e + 1);
                }
            }
            i = abs + 1;
        } else {
            break;
        }
    }
    None
}

fn cell_text(chunk: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    while i < chunk.len() {
        if chunk.as_bytes()[i] == b'<' {
            let Some(end) = tag_end(chunk, i) else { break };
            let tag = &chunk[i + 1..end];
            let name = tag
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if name == "br" {
                out.push(' ');
            }
            i = end + 1;
        } else {
            let next = chunk[i..].find('<').map(|p| i + p).unwrap_or(chunk.len());
            out.push_str(&chunk[i..next]);
            i = next;
        }
    }
    collapse_ws(&decode_entities(&out)).trim().to_string()
}

fn parse_table(chunk: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let lower = chunk.to_ascii_lowercase();
    let mut pos = 0usize;
    while let Some(p) = lower[pos..].find("<tr") {
        let row_start = pos + p;
        let row_end = lower[row_start..]
            .find("</tr>")
            .map(|e| row_start + e)
            .unwrap_or_else(|| lower.len());
        let row_html = &chunk[row_start..row_end];
        let row_lower = row_html.to_ascii_lowercase();
        let mut cells = Vec::new();
        let mut cpos = 0usize;
        loop {
            let td = row_lower[cpos..].find("<td").map(|x| (cpos + x, 3));
            let th = row_lower[cpos..].find("<th").map(|x| (cpos + x, 3));
            let pick = match (td, th) {
                (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
            let Some((start, _)) = pick else { break };
            let Some(gt) = tag_end(row_html, start) else { break };
            let content_start = gt + 1;
            let rest_lower = &row_lower[content_start..];
            let next = ["<td", "<th", "</tr"]
                .iter()
                .filter_map(|n| rest_lower.find(n))
                .min()
                .map(|x| content_start + x)
                .unwrap_or(row_html.len());
            cells.push(cell_text(&row_html[content_start..next]));
            cpos = next.max(content_start);
        }
        if !cells.is_empty() {
            rows.push(cells);
        }
        pos = row_end.saturating_add(5);
        if pos >= lower.len() {
            break;
        }
    }
    rows
}

fn flush(spans: &mut Vec<Span>, blocks: &mut Vec<Block>) {
    if spans.is_empty() {
        return;
    }
    if let Some(first) = spans.first_mut() {
        first.text = first.text.trim_start().to_string();
    }
    if let Some(last) = spans.last_mut() {
        last.text = last.text.trim_end().to_string();
    }
    spans.retain(|s| !s.text.is_empty());
    if !spans.is_empty() {
        blocks.push(Block::Para(std::mem::take(spans)));
    }
}

pub fn parse(html: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut style = Style::default();
    let mut i = 0usize;

    while i < html.len() {
        if html.as_bytes()[i] == b'<' {

            if html[i..].starts_with("<!--") {
                i = html[i..]
                    .find("-->")
                    .map(|p| i + p + 3)
                    .unwrap_or(html.len());
                continue;
            }
            let Some(end) = tag_end(html, i) else { break };
            let tag = html[i + 1..end].trim().to_string();
            let closing = tag.starts_with('/');
            let name = tag
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();

            if !closing && name == "img" {
                flush(&mut spans, &mut blocks);
                if let Some(src) = attr(&tag, "src") {
                    if !src.trim().is_empty() {
                        blocks.push(Block::Image(src.trim().to_string()));
                    }
                }
                i = end + 1;
                continue;
            }

            if !closing && name == "table" {
                flush(&mut spans, &mut blocks);
                let table_end = find_table_end(html, i).unwrap_or(html.len());
                let rows = parse_table(&html[i..table_end]);
                if !rows.is_empty() {
                    blocks.push(Block::Table(rows));
                }
                i = table_end;
                continue;
            }

            if !closing && SKIP_TAGS.contains(&name.as_str()) {
                i = skip_to_close(html, end + 1, &name);
                continue;
            }

            if BLOCK_TAGS.contains(&name.as_str()) {
                flush(&mut spans, &mut blocks);
                i = end + 1;
                continue;
            }

            match name.as_str() {
                "strong" | "b" => style.bold = !closing,
                "em" | "i" => style.italic = !closing,
                "u" => style.underline = !closing,
                "sup" => style.sup = !closing,
                "sub" => style.sub = !closing,
                "latex" => style.latex = !closing,
                _ => {}
            }
            i = end + 1;
        } else {
            let next = html[i..].find('<').map(|p| i + p).unwrap_or(html.len());
            let raw = &html[i..next];
            let text = collapse_ws(&decode_entities(raw));
            if !text.is_empty() {
                spans.push(style.span(text));
            }
            i = next;
        }
    }
    flush(&mut spans, &mut blocks);
    blocks
}

pub fn plain_text(html: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    while i < html.len() {
        if html.as_bytes()[i] == b'<' {
            if html[i..].starts_with("<!--") {
                i = html[i..].find("-->").map(|p| i + p + 3).unwrap_or(html.len());
                continue;
            }
            let Some(end) = tag_end(html, i) else { break };
            let tag = html[i + 1..end].trim().to_string();
            let name = tag
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !tag.starts_with('/') && SKIP_TAGS.contains(&name.as_str()) {
                i = skip_to_close(html, end + 1, &name);
                continue;
            }
            if matches!(name.as_str(), "br" | "p" | "div" | "li" | "tr") {
                out.push(' ');
            }
            i = end + 1;
        } else {
            let next = html[i..].find('<').map(|p| i + p).unwrap_or(html.len());
            out.push_str(&html[i..next]);
            i = next;
        }
    }
    collapse_ws(&decode_entities(&out)).trim().to_string()
}

fn percent_encode_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    for b in p.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '/' | ':' | '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn file_uri(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    format!("file:///{}", percent_encode_path(s.trim_start_matches('/')))
}

fn add_span(ui: &mut egui::Ui, span: &Span) {
    let mut text = egui::RichText::new(span.text.trim());
    if text.text().is_empty() {
        return;
    }
    if span.bold {
        text = text.strong();
    }
    if span.italic {
        text = text.italics();
    }
    if span.underline {
        text = text.underline();
    }
    if span.latex {
        let formula = span.text.trim();
        if let Some(dir) = latex_dir() {
            let svg = dir.join(format!("{}.svg", sha1_hex(formula.as_bytes())));
            if svg.is_file() {
                ui.add(
                    egui::Image::new(egui::ImageSource::Uri(file_uri(&svg).into()))
                        .max_height(26.0)
                        .maintain_aspect_ratio(true),
                );
                return;
            }
        }
        text = text
            .monospace()
            .color(egui::Color32::from_rgb(160, 232, 205))
            .background_color(egui::Color32::from_rgba_unmultiplied(88, 205, 168, 22));
    } else if span.sup || span.sub {
        text = text.size(ui.style().text_styles[&egui::TextStyle::Body].size * 0.72);
    }
    ui.label(text);
}

pub fn render(ui: &mut egui::Ui, blocks: &[Block], root: &Path) {
    for (bi, block) in blocks.iter().enumerate() {
        match block {
            Block::Para(spans) => {
                ui.horizontal_wrapped(|ui| {
                    for s in spans {
                        add_span(ui, s);
                    }
                });
            }
            Block::Image(src) => {
                let path = if src.starts_with("http://") || src.starts_with("https://") {
                    None
                } else {
                    let rel = src.trim_start_matches('/');
                    let p = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
                    p.exists().then_some(p)
                };
                match path {
                    Some(p) => {
                        ui.add(
                            egui::Image::new(egui::ImageSource::Uri(file_uri(&p).into()))
                                .max_width(ui.available_width().min(760.0))
                                .max_height(420.0)
                                .maintain_aspect_ratio(true),
                        );
                    }
                    None => {
                        ui.label(
                            egui::RichText::new(format!("[изображение: {src}]"))
                                .italics()
                                .weak(),
                        );
                    }
                }
            }
            Block::Table(rows) => {
                ui.push_id(("table", bi), |ui| {
                    egui::Grid::new(("grid", bi))
                        .striped(true)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            for row in rows {
                                for cell in row {
                                    ui.label(egui::RichText::new(cell).size(14.0));
                                }
                                ui.end_row();
                            }
                        });
                });
            }
        }
        ui.add_space(4.0);
    }
}
