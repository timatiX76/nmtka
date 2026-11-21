use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub marker: String,
    pub html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub title: Option<String>,
    pub answers: Vec<Answer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRef {
    pub url: String,

    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRef {
    pub test_id: serde_json::Value,
    pub order: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    SingleChoice,
    MultiChoice,
    ShortAnswer,
    Matching,
    Sequence,
    #[serde(other)]
    Unknown,
}

impl QuestionType {
    pub fn label(self) -> &'static str {
        match self {
            QuestionType::SingleChoice => "один варіант",
            QuestionType::MultiChoice => "кілька варіантів",
            QuestionType::ShortAnswer => "коротка відповідь",
            QuestionType::Matching => "відповідність",
            QuestionType::Sequence => "послідовність",
            QuestionType::Unknown => "завдання",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: u32,
    pub subject: String,
    pub tip: u8,
    #[serde(rename = "type")]
    pub kind: QuestionType,
    pub out_order: u32,
    pub text: String,
    pub question_html: String,
    pub answers: Vec<Answer>,
    #[serde(default)]
    pub groups: Option<Vec<Group>>,
    #[serde(default)]
    pub answer_table_html: Option<String>,

    pub correct: Vec<String>,
    #[serde(default)]
    pub correct_raw: Option<String>,
    #[serde(default)]
    pub input: bool,
    #[serde(default)]
    pub input_maxlength: Option<u32>,
    #[serde(default)]
    pub explanation_html: Option<String>,
    #[serde(default)]
    pub explanation_text: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub latex: Vec<String>,
    #[serde(default)]
    pub images: Vec<ImageRef>,
    #[serde(default)]
    pub source_test: Option<u32>,
    #[serde(default)]
    pub tests: Vec<TestRef>,
}

impl Question {

    pub fn correct_marker(&self) -> Option<&str> {
        let v = self.correct.first()?;
        let idx = v.bytes().next()?;
        if !(b'a'..=b'e').contains(&idx) {
            return None;
        }
        self.answers
            .get((idx - b'a') as usize)
            .map(|a| a.marker.as_str())
    }

    pub fn correct_index(&self) -> Option<usize> {
        let v = self.correct.first()?;
        let idx = v.bytes().next()?;
        if !(b'a'..=b'e').contains(&idx) {
            return None;
        }
        let i = (idx - b'a') as usize;
        if i < self.answers.len() {
            Some(i)
        } else {
            None
        }
    }

    pub fn correct_indices(&self) -> Vec<usize> {
        self.correct
            .iter()
            .filter_map(|v| {
                let b = v.bytes().next()?;
                if !(b'a'..=b'e').contains(&b) {
                    return None;
                }
                let i = (b - b'a') as usize;
                (i < self.answers.len()).then_some(i)
            })
            .collect()
    }

    pub fn correct_pairs(&self) -> Vec<(char, char)> {
        self.correct
            .iter()
            .filter_map(|s| {
                let b = s.as_bytes();
                (b.len() == 2).then(|| (b[0] as char, b[1] as char))
            })
            .collect()
    }

    pub fn correct_text(&self) -> Option<&str> {
        self.correct.first().map(|s| s.as_str())
    }

    pub fn has_explanation(&self) -> bool {
        self.explanation_html.is_some() || self.explanation_text.is_some()
    }

    pub fn explanation_is_stub(&self) -> bool {
        self.explanation_text
            .as_deref()
            .map(|t| t.to_uppercase().contains("ПРЕМІУМ"))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Test {
    pub test_id: serde_json::Value,
    #[serde(default)]
    pub id: Option<u32>,
    pub subject: String,
    #[serde(default)]
    pub subject_name: Option<String>,
    pub title: String,
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub session: Option<String>,
    #[serde(default)]
    pub total: Option<u32>,
    pub questions_count: u32,
    pub questions: Vec<Question>,
    pub url: String,
}

impl Test {
    pub fn key(&self) -> String {
        match &self.test_id {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestListItem {
    pub id: u32,
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub session: Option<String>,
    #[serde(default)]
    pub questions_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestIndex {
    pub subject: String,
    pub name: String,
    pub tests: Vec<TestListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub slug: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicList {
    pub subject: String,
    pub topics: Vec<Topic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NmtVariant {
    pub slug: String,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub questions_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NmtList {
    pub subject: String,
    pub variants: Vec<NmtVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectStat {
    pub slug: String,
    pub name: String,
    pub tests: u32,
    pub questions: u32,
    #[serde(default)]
    pub topics: u32,
    #[serde(default)]
    pub nmt_variants: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetIndex {
    pub subjects: Vec<SubjectStat>,
    #[serde(default)]
    pub tests: u32,
    #[serde(default)]
    pub questions: u32,
}
