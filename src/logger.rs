use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

struct State {
    file: Option<File>,
    size: u64,
}

pub struct Logger {
    path: String,
    max_bytes: u64,
    max_files: u32,
    max_body: usize,
    state: Mutex<State>,
}

impl Logger {
    pub fn new(path: &str, max_bytes: u64, max_files: u32, max_body: usize) -> Self {
        let (file, size) = Self::open(path);
        Self {
            path: path.to_string(),
            max_bytes,
            max_files,
            max_body,
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
        let cut = &body[..body.len().min(self.max_body)];
        let mut text = format!("[{}] #{id} {head} ({} octets)\n", timestamp(), body.len());
        text.push_str(&String::from_utf8_lossy(cut));
        if cut.len() < body.len() {
            text.push_str("\n...[tronqué]");
        }
        text.push_str("\n\n");

        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.size > 0 && st.size + text.len() as u64 > self.max_bytes {
            self.rotate(&mut st);
        }
        if let Some(f) = st.file.as_mut() {
            if f.write_all(text.as_bytes()).is_ok() {
                st.size += text.len() as u64;
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

// UTC, sans dépendance externe (algorithme civil_from_days).
fn timestamp() -> String {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let (secs, ms) = (d.as_secs() as i64, d.subsec_millis());
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}.{ms:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}
