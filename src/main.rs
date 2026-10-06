mod config;
mod locale;
mod logger;
mod program;
mod proxy;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use locale::{Message, text};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cfg = config::Config::load();
    let listener = match TcpListener::bind(&cfg.listen).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "{} {}: {e}",
                text(cfg.language, Message::ListenFailed),
                cfg.listen
            );
            std::process::exit(1);
        }
    };
    println!(
        "{}: {} -> {}",
        text(cfg.language, Message::ServerListening),
        cfg.listen,
        cfg.target
    );
    let language = cfg.language;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let program_task = tokio::spawn(program::run(
        cfg.managed_program_enabled,
        cfg.managed_program_path.clone(),
        cfg.managed_program_args.clone(),
        cfg.managed_program_restart_interval_secs,
        cfg.language,
        shutdown_rx,
    ));
    let ctx = Arc::new(proxy::Ctx::new(cfg));

    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);
    loop {
        tokio::select! {
            signal = &mut ctrl_c => {
                match signal {
                    Ok(()) => println!("{}", text(language, Message::ShutdownRequested)),
                    Err(error) => eprintln!("{}: {error}", text(language, Message::ShutdownSignalFailed)),
                }
                break;
            }
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else {
                    continue;
                };
                let _ = stream.set_nodelay(true);
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    let svc = service_fn(move |req| proxy::handle(ctx.clone(), req));
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), svc)
                        .await;
                });
            }
        }
    }

    let _ = shutdown_tx.send(());
    if let Err(error) = program_task.await {
        eprintln!("{}: {error}", text(language, Message::ProgramTaskFailed));
    }
}
