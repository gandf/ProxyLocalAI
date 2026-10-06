use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::sleep;

pub async fn run(enabled: bool, path: String, args: Vec<String>, restart_interval_secs: u64) {
    if !enabled {
        return;
    }
    if path.trim().is_empty() {
        eprintln!("Programme configuré mais aucun chemin n'est renseigné");
        return;
    }

    let interval = Duration::from_secs(restart_interval_secs);
    let mut child = start(&path, &args);

    if interval.is_zero() {
        if let Some(mut child) = child {
            if let Err(error) = child.wait().await {
                eprintln!("Erreur en attendant le programme {path}: {error}");
            }
        }
        return;
    }

    loop {
        sleep(interval).await;

        if let Some(mut running) = child.take() {
            match running.try_wait() {
                Ok(Some(status)) => println!("Programme {path} terminé ({status}); relancement"),
                Ok(None) => {
                    println!("Relancement du programme {path}");
                    if let Err(error) = running.start_kill() {
                        eprintln!("Impossible d'arrêter le programme {path}: {error}");
                    }
                    if let Err(error) = running.wait().await {
                        eprintln!("Erreur en attendant l'arrêt du programme {path}: {error}");
                    }
                }
                Err(error) => {
                    eprintln!("Impossible de vérifier le programme {path}: {error}");
                    if let Err(error) = running.start_kill() {
                        eprintln!("Impossible d'arrêter le programme {path}: {error}");
                    }
                    let _ = running.wait().await;
                }
            }
        }

        child = start(&path, &args);
    }
}

fn start(path: &str, args: &[String]) -> Option<Child> {
    match Command::new(path).args(args).kill_on_drop(true).spawn() {
        Ok(child) => {
            println!("Programme démarré: {path}");
            Some(child)
        }
        Err(error) => {
            eprintln!("Impossible de démarrer le programme {path}: {error}");
            None
        }
    }
}
