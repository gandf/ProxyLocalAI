use crate::locale::{text, Language, Message};
use chrono::Local;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

struct State {
    file: Option<File>,
    size: u64,
}

pub struct Logger {
    enabled: bool,
    path: String,
    max_bytes: u64,
    max_files: u32,
    max_body: usize,
    language: Language,
    state: Mutex<State>,
}

impl Logger {
    pub fn new(
        path: &str,
        max_bytes: u64,
        max_files: u32,
        max_body: usize,
        enabled: bool,
        language: Language,
    ) -> Self {
        let (file, size) = if enabled { Self::open(path) } else { (None, 0) };
        Self {
            enabled,
            path: path.to_string(),
            max_bytes,
            max_files,
            max_body,
            language,
            state: Mutex::new(State { file, size }),
        }
    }

    fn open(path: &str) -> (Option<File>, u64) {
        let f = OpenOptions::new().create(true).append(true).open(path).ok();
        let size = f.as_ref().and_then(|f| f.metadata().ok()).map_or(0, |m| m.len());
        (f, size)
    }

    pub fn max_body(&self) -> usize {
        self.max_body
    }

    pub fn log(&self, id: u64, head: &str, body: &[u8]) {
        if !self.enabled {
            return;
        }

        let cut = &body[..body.len().min(self.max_body)];
        let mut entry = format!(
            "[{}] #{id} {head} ({} {})\n",
            timestamp(),
            body.len(),
            text(self.language, Message::Bytes)
        );
        entry.push_str(&String::from_utf8_lossy(cut));
        if cut.len() < body.len() {
            entry.push_str(&format!("\n...[{}]", text(self.language, Message::Truncated)));
        }
        entry.push_str("\n\n");

        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.size > 0 && st.size + entry.len() as u64 > self.max_bytes {
            self.rotate(&mut st);
        }
        if let Some(f) = st.file.as_mut() {
            if f.write_all(entry.as_bytes()).is_ok() {
                st.size += entry.len() as u64;
            }
        }
    }

    fn rotate(&self, st: &mut State) {
        st.file = None;
        if self.max_files == 0 {
            let _ = fs::remove_file(&self.path);
        } else {
            let _ = fs::remove_file(format!("{}.{}", self.path, self.max_files));
            for i in (1..self.max_files).rev() {
                let _ = fs::rename(format!("{}.{i}", self.path), format!("{}.{}", self.path, i + 1));
            }
            let _ = fs::rename(&self.path, format!("{}.1", self.path));
        }
        let (f, s) = Self::open(&self.path);
        st.file = f;
        st.size = s;
    }
}

fn timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S%.3f%:z").to_string()
}
