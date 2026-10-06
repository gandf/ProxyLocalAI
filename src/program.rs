use crate::locale::{Language, Message, text};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::{RwLock, oneshot};
use tokio::time::sleep;

#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

pub async fn run(
    enabled: bool,
    path: String,
    args: Vec<String>,
    restart_interval_secs: u64,
    language: Language,
    restart_gate: Arc<RwLock<()>>,
    mut shutdown: oneshot::Receiver<()>,
) {
    if !enabled {
        return;
    }
    if path.trim().is_empty() {
        eprintln!("{}", text(language, Message::ProgramPathMissing));
        return;
    }

    let interval = Duration::from_secs(restart_interval_secs);
    let mut child = start(&path, &args, language);

    if interval.is_zero() {
        let _ = shutdown.await;
        if let Some(mut child) = child {
            stop_tree(&mut child, &path, language).await;
        }
        return;
    }

    loop {
        tokio::select! {
            _ = sleep(interval) => {}
            _ = &mut shutdown => {
                if let Some(mut child) = child {
                    stop_tree(&mut child, &path, language).await;
                }
                return;
            }
        }

        let _restart_guard = restart_gate.write().await;
        if let Some(mut running) = child.take() {
            match running.try_wait() {
                Ok(Some(status)) => println!(
                    "{} {path} ({status}) {}",
                    text(language, Message::Program),
                    text(language, Message::ProgramExitedRestarting)
                ),
                Ok(None) => {
                    println!("{} {path}", text(language, Message::ProgramRestarting));
                    stop_tree(&mut running, &path, language).await;
                }
                Err(error) => {
                    eprintln!(
                        "{} {path}: {error}",
                        text(language, Message::ProgramCheckFailed)
                    );
                    stop_tree(&mut running, &path, language).await;
                }
            }
        }

        child = start(&path, &args, language);
    }
}

fn start(path: &str, args: &[String], language: Language) -> Option<Child> {
    let mut command = if is_batch_file(path) {
        let mut command = Command::new("cmd.exe");
        command.arg("/C").arg(path).args(args);
        command
    } else {
        let mut command = Command::new(path);
        command.args(args);
        command
    };

    #[cfg(windows)]
    command.creation_flags(CREATE_NEW_PROCESS_GROUP);

    match command.kill_on_drop(true).spawn() {
        Ok(child) => {
            println!("{}: {path}", text(language, Message::ProgramStarted));
            Some(child)
        }
        Err(error) => {
            eprintln!(
                "{} {path}: {error}",
                text(language, Message::ProgramStartFailed)
            );
            None
        }
    }
}

fn is_batch_file(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd")
    })
}

async fn stop_tree(child: &mut Child, path: &str, language: Language) {
    let Some(pid) = child.id() else {
        return;
    };

    println!("{}: {path}", text(language, Message::ProgramTreeStopping));

    #[cfg(windows)]
    let result = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output()
        .await;

    #[cfg(not(windows))]
    let result = child.start_kill().map(|_| ());

    let tree_stopped = match result {
        #[cfg(windows)]
        Ok(output) if output.status.success() => true,
        #[cfg(windows)]
        Ok(output) => {
            let detail = if output.stderr.is_empty() {
                &output.stdout
            } else {
                &output.stderr
            };
            eprintln!(
                "{}: taskkill exited with {}: {}",
                text(language, Message::ProgramTreeStopFailed),
                output.status,
                String::from_utf8_lossy(detail).trim()
            );
            false
        }
        #[cfg(not(windows))]
        Ok(()) => true,
        Err(error) => {
            eprintln!(
                "{}: {error}",
                text(language, Message::ProgramTreeStopFailed)
            );
            false
        }
    };

    if !tree_stopped && child.try_wait().ok().flatten().is_none() {
        if let Err(error) = child.start_kill() {
            eprintln!(
                "{} {path}: {error}",
                text(language, Message::ProgramStopFailed)
            );
        }
    }

    if let Err(error) = child.wait().await {
        eprintln!(
            "{} {path}: {error}",
            text(language, Message::ProgramStopWaitFailed)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{is_batch_file, start, stop_tree};
    use crate::locale::Language;
    use std::time::{Duration, Instant};

    #[test]
    fn detects_batch_files_case_insensitively() {
        assert!(is_batch_file("F:/Strata/START-HERE.bat"));
        assert!(is_batch_file("F:/Tools/start.CMD"));
        assert!(!is_batch_file("F:/Tools/worker.exe"));
    }

    #[cfg(windows)]
    #[tokio::test(flavor = "current_thread")]
    async fn force_stops_a_batch_waiting_for_confirmation() {
        let path =
            std::env::temp_dir().join(format!("proxyia-stop-test-{}.bat", std::process::id()));
        std::fs::write(&path, "@echo off\r\npause\r\n").unwrap();
        let path = path.to_string_lossy().into_owned();
        let mut child = start(&path, &[], Language::En).expect("batch should start");
        tokio::time::sleep(Duration::from_millis(250)).await;

        let started = Instant::now();
        stop_tree(&mut child, &path, Language::En).await;

        assert!(started.elapsed() < Duration::from_secs(5));
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(windows)]
    #[tokio::test(flavor = "current_thread")]
    async fn cmd_config_runs_a_batch_with_a_quoted_working_directory() {
        let dir = std::env::temp_dir().join(format!("proxyia batch test {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let batch = dir.join("START-HERE.bat");
        std::fs::write(
            &batch,
            "@echo off\r\ncd /d \"%~dp0\"\r\nif errorlevel 1 exit /b 1\r\nexit /b 0\r\n",
        )
        .unwrap();

        let args = ["/C".to_owned(), batch.to_string_lossy().into_owned()];
        let mut child = start("C:\\Windows\\System32\\cmd.exe", &args, Language::En)
            .expect("cmd.exe should start");
        let status = child.wait().await.unwrap();

        std::fs::remove_dir_all(dir).unwrap();
        assert!(status.success(), "cmd.exe exited with {status}");
    }
}
