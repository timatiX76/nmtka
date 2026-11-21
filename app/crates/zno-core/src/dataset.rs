use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::models::*;

pub struct Dataset {
    root: PathBuf,
    index: DatasetIndex,
}

impl Dataset {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let index_path = root.join("data").join("index.json");
        let text = fs::read_to_string(&index_path)
            .with_context(|| format!("не читается {}", index_path.display()))?;
        let index: DatasetIndex = serde_json::from_str(&text)
            .with_context(|| format!("не парсится {}", index_path.display()))?;
        Ok(Self { root, index })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn index(&self) -> &DatasetIndex {
        &self.index
    }

    pub fn subjects(&self) -> &[SubjectStat] {
        &self.index.subjects
    }

    pub fn subject(&self, slug: &str) -> Option<&SubjectStat> {
        self.index.subjects.iter().find(|s| s.slug == slug)
    }

    fn read_json<T: serde::de::DeserializeOwned>(&self, rel: &Path) -> Result<T> {
        let p = self.root.join(rel);
        let text = fs::read_to_string(&p).with_context(|| format!("не читается {}", p.display()))?;
        serde_json::from_str(&text).with_context(|| format!("не парсится {}", p.display()))
    }

    pub fn test_index(&self, subject: &str) -> Result<TestIndex> {
        self.read_json(Path::new("data").join("tests").join(subject).join("_index.json").as_path())
    }

    pub fn topics(&self, subject: &str) -> Result<Option<TopicList>> {
        let p = self
            .root
            .join("data")
            .join("topics")
            .join(format!("{subject}.json"));
        if !p.exists() {
            return Ok(None);
        }
        Ok(Some(self.read_json(
            Path::new("data").join("topics").join(format!("{subject}.json")).as_path(),
        )?))
    }

    pub fn nmt(&self, subject: &str) -> Result<Option<NmtList>> {
        let p = self
            .root
            .join("data")
            .join("nmt")
            .join(format!("{subject}.json"));
        if !p.exists() {
            return Ok(None);
        }
        Ok(Some(self.read_json(
            Path::new("data").join("nmt").join(format!("{subject}.json")).as_path(),
        )?))
    }

    pub fn load_test(&self, subject: &str, key: &str) -> Result<Test> {
        let in_tests = Path::new("data").join("tests").join(subject).join(format!("{key}.json"));
        if self.root.join(&in_tests).exists() {
            return self.read_json(&in_tests);
        }
        let in_nmt = Path::new("data").join("nmt").join(subject).join(format!("{key}.json"));
        if self.root.join(&in_nmt).exists() {
            return self.read_json(&in_nmt);
        }
        bail!("тест {subject}/{key} не найден")
    }

    pub fn load_questions(&self, subject: &str) -> Result<Vec<Question>> {
        let p = self
            .root
            .join("data")
            .join("questions")
            .join(format!("{subject}.jsonl"));
        let f = fs::File::open(&p).with_context(|| format!("не открывается {}", p.display()))?;
        let mut out = Vec::new();
        for (n, line) in BufReader::new(f).lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let q: Question = serde_json::from_str(&line)
                .with_context(|| format!("строка {} в {}", n + 1, p.display()))?;
            out.push(q);
        }
        Ok(out)
    }

    pub fn load_topic_questions(&self, subject: &str, slug: &str) -> Result<Vec<Question>> {
        let p = self
            .root
            .join("data")
            .join("tags")
            .join(subject)
            .join(format!("{slug}.json"));
        let text = fs::read_to_string(&p).with_context(|| format!("не читается {}", p.display()))?;
        let v: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("не парсится {}", p.display()))?;
        let qs = v
            .get("questions")
            .cloned()
            .unwrap_or(serde_json::Value::Array(Vec::new()));
        serde_json::from_value(qs).with_context(|| format!("вопросы в {}", p.display()))
    }

    pub fn load_bank_sample(&self, subject: &str, n: usize) -> Result<Vec<Question>> {
        let mut all = self.load_questions(subject)?;
        if all.len() < 2 {
            all.truncate(n);
            return Ok(all);
        }
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
        for i in (1..all.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let j = (seed % (i as u64 + 1)) as usize;
            all.swap(i, j);
        }
        all.truncate(n);
        Ok(all)
    }

    pub fn image_path(&self, rel: &str) -> PathBuf {
        self.root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
    }
}
