use crate::locale::{text, Language, Message};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::sleep;

pub async fn run(
    enabled: bool,
    path: String,
    args: Vec<String>,
    restart_interval_secs: u64,
    language: Language,
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
        if let Some(mut child) = child {
            if let Err(error) = child.wait().await {
                eprintln!("{} {path}: {error}", text(language, Message::ProgramWaitFailed));
            }
        }
        return;
    }

    loop {
        sleep(interval).await;

        if let Some(mut running) = child.take() {
            match running.try_wait() {
                Ok(Some(status)) => println!(
                    "{} {path} ({status}) {}",
                    text(language, Message::Program),
                    text(language, Message::ProgramExitedRestarting)
                ),
                Ok(None) => {
                    println!("{} {path}", text(language, Message::ProgramRestarting));
                    if let Err(error) = running.start_kill() {
                        eprintln!("{} {path}: {error}", text(language, Message::ProgramStopFailed));
                    }
                    if let Err(error) = running.wait().await {
                        eprintln!("{} {path}: {error}", text(language, Message::ProgramStopWaitFailed));
                    }
                }
                Err(error) => {
                    eprintln!("{} {path}: {error}", text(language, Message::ProgramCheckFailed));
                    if let Err(error) = running.start_kill() {
                        eprintln!("{} {path}: {error}", text(language, Message::ProgramStopFailed));
                    }
                    let _ = running.wait().await;
                }
            }
        }

        child = start(&path, &args, language);
    }
}

fn start(path: &str, args: &[String], language: Language) -> Option<Child> {
    match Command::new(path).args(args).kill_on_drop(true).spawn() {
        Ok(child) => {
            println!("{}: {path}", text(language, Message::ProgramStarted));
            Some(child)
        }
        Err(error) => {
            eprintln!("{} {path}: {error}", text(language, Message::ProgramStartFailed));
            None
        }
    }
}
