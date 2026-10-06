mod config;
mod logger;
mod program;
mod proxy;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cfg = config::Config::load();
    let listener = match TcpListener::bind(&cfg.listen).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Impossible d'écouter sur {}: {e}", cfg.listen);
            std::process::exit(1);
        }
    };
    println!("proxyia: {} -> {}", cfg.listen, cfg.target);
    tokio::spawn(program::run(
        cfg.managed_program_enabled,
        cfg.managed_program_path.clone(),
        cfg.managed_program_args.clone(),
        cfg.managed_program_restart_interval_secs,
    ));
    let ctx = Arc::new(proxy::Ctx::new(cfg));

    loop {
        let Ok((stream, _)) = listener.accept().await else {
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
