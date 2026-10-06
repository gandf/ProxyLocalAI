use crate::config::{Config, RequestReplacement};
use crate::logger::Logger;
use bytes::Bytes;
use http_body_util::{channel::Channel, combinators::BoxBody, BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{HeaderMap, HeaderName, CONTENT_TYPE, HOST};
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;
use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};

type PBody = BoxBody<Bytes, Infallible>;

pub struct Ctx {
    cfg: Config,
    logger: Arc<Logger>,
    client: Client<HttpConnector, Full<Bytes>>,
    counter: AtomicU64,
}

impl Ctx {
    pub fn new(cfg: Config) -> Self {
        let logger = Arc::new(Logger::new(
            &cfg.log_file,
            cfg.log_max_bytes,
            cfg.log_max_files,
            cfg.max_body_log_bytes,
            cfg.log_enabled,
        ));
        Self {
            cfg,
            logger,
            client: Client::builder(TokioExecutor::new()).build_http(),
            counter: AtomicU64::new(1),
        }
    }
}

fn full(b: Bytes) -> PBody {
    Full::new(b).map_err(|e| match e {}).boxed()
}

fn simple(status: StatusCode, msg: &str) -> Response<PBody> {
    let mut r = Response::new(full(Bytes::from(msg.to_string())));
    *r.status_mut() = status;
    r
}

fn skip_header(name: &HeaderName) -> bool {
    let is_skip: bool = matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-connection"
            | "transfer-encoding"
            | "upgrade"
            | "te"
            | "trailer"
            | "content-length"
            | "host"
            | "accept-encoding"
    );
    is_skip
}

fn filtered(h: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (k, v) in h {
        if !skip_header(k) {
            out.append(k.clone(), v.clone());
        }
    }
    out
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= hay.len() {
        return None;
    }
    hay[from..].windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

// Vrai si un champ de génération (content, tool_calls...) est non vide.
fn has_content(b: &[u8]) -> bool {
    for key in [
        &b"\"content\""[..],
        b"\"reasoning_content\"",
        b"\"text\"",
        b"\"tool_calls\"",
        b"\"function_call\"",
    ] {
        let mut from = 0;
        while let Some(p) = find(b, key, from) {
            from = p + key.len();
            let mut i = from;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= b.len() || b[i] != b':' {
                continue;
            }
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            let (Some(&c), next) = (b.get(i), b.get(i + 1)) else {
                continue;
            };
            match c {
                b'"' if next != Some(&b'"') => return true,
                b'[' if next != Some(&b']') => return true,
                b'{' => return true,
                _ => {}
            }
        }
    }
    false
}

fn is_empty_response(b: &[u8]) -> bool {
    b.iter().all(|c| c.is_ascii_whitespace())
        || (find(b, b"\"choices\"", 0).is_some() && !has_content(b))
}

pub fn apply_request_replacements_with_change(body: &[u8], rules: &[RequestReplacement]) -> (Bytes, bool) {
    if rules.is_empty() {
        return (Bytes::copy_from_slice(body), false);
    }

    let mut text = String::from_utf8_lossy(body).into_owned();
    let mut changed = false;
    for rule in rules {
        if rule.from.is_empty() {
            continue;
        }

        if rule.first_only {
            let before = text.clone();
            let pos = if rule.from_end {
                text.rfind(&rule.from)
            } else {
                text.find(&rule.from)
            };
            if let Some(pos) = pos {
                let end = pos + rule.from.len();
                text.replace_range(pos..end, &rule.to);
            }
            if text != before {
                changed = true;
            }
        } else {
            let replaced = text.replace(&rule.from, &rule.to);
            if replaced != text {
                changed = true;
            }
            text = replaced;
        }
    }
    (Bytes::from(text), changed)
}

#[allow(dead_code)]
pub fn apply_request_replacements(body: &[u8], rules: &[RequestReplacement]) -> Bytes {
    apply_request_replacements_with_change(body, rules).0
}

pub async fn handle(ctx: Arc<Ctx>, req: Request<Incoming>) -> Result<Response<PBody>, Infallible> {
    let id = ctx.counter.fetch_add(1, Ordering::Relaxed);
    let (parts, body) = req.into_parts();
    let body = match body.collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return Ok(simple(StatusCode::BAD_REQUEST, "invalid request body")),
    };
    let original_body = body.clone();
    let (body, changed) = apply_request_replacements_with_change(&body, &ctx.cfg.request_replacements);
    ctx.logger.log(id, &format!("REQ {} {} (reçue)", parts.method, parts.uri), &original_body);
    if changed {
        ctx.logger.log(id, &format!("REQ {} {} (après remplacements)", parts.method, parts.uri), &body);
    }

    let pq = parts.uri.path_and_query().map_or("/", |p| p.as_str());
    let uri = format!("http://{}{}", ctx.cfg.target, pq);
    let headers = filtered(&parts.headers);
    let delay = Duration::from_millis(ctx.cfg.retry_delay_ms);

    for attempt in 0..=ctx.cfg.max_retries {
        let last = attempt == ctx.cfg.max_retries;
        let mut rb = Request::builder().method(parts.method.clone()).uri(&uri);
        if let Some(h) = rb.headers_mut() {
            *h = headers.clone();
        }
        let rb = rb.header(HOST, ctx.cfg.target.as_str());
        let Ok(r) = rb.body(Full::new(body.clone())) else {
            return Ok(simple(StatusCode::BAD_REQUEST, "invalid request"));
        };

        let wait = Duration::from_secs(ctx.cfg.request_timeout_s);
        let resp = match timeout(wait, ctx.client.request(r)).await {
            Ok(Ok(r)) => r,
            other => {
                let err = match other {
                    Ok(Err(e)) => e.to_string(),
                    _ => "timeout".into(),
                };
                ctx.logger.log(id, &format!("ERR tentative {attempt}"), err.as_bytes());
                if last {
                    return Ok(simple(StatusCode::BAD_GATEWAY, "upstream unavailable"));
                }
                sleep(delay).await;
                continue;
            }
        };

        if let Some(r) = relay(&ctx, id, attempt, last, resp).await {
            return Ok(r);
        }
        sleep(delay).await;
    }
    Ok(simple(StatusCode::BAD_GATEWAY, "upstream unavailable"))
}

// None = réponse vide, la requête doit être renvoyée.
async fn relay(
    ctx: &Arc<Ctx>,
    id: u64,
    attempt: u32,
    last: bool,
    resp: Response<Incoming>,
) -> Option<Response<PBody>> {
    let (parts, mut body) = resp.into_parts();
    let ok = parts.status.is_success();
    let sse = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/event-stream"));
    let head = format!("RES {} tentative {attempt}", parts.status);
    let build = |b: PBody| {
        let mut r = Response::new(b);
        *r.status_mut() = parts.status;
        *r.headers_mut() = filtered(&parts.headers);
        r
    };

    if !sse {
        let bytes = match body.collect().await {
            Ok(b) => b.to_bytes(),
            Err(e) => {
                ctx.logger.log(id, &format!("ERR tentative {attempt}"), e.to_string().as_bytes());
                return if last {
                    Some(simple(StatusCode::BAD_GATEWAY, "upstream error"))
                } else {
                    None
                };
            }
        };
        let empty = ok && is_empty_response(&bytes);
        let tag = if empty { " VIDE" } else { "" };
        ctx.logger.log(id, &format!("{head}{tag}"), &bytes);
        if empty && !last {
            return None;
        }
        return Some(build(full(bytes)));
    }

    // SSE : on retient les chunks jusqu'au premier contenu utile.
    let mut held: Vec<u8> = Vec::new();
    let mut found = false;
    while let Some(Ok(frame)) = body.frame().await {
        if let Ok(d) = frame.into_data() {
            held.extend_from_slice(&d);
            if has_content(&held) {
                found = true;
                break;
            }
        }
    }

    if !found {
        let retry = ok && !last;
        let tag = if ok { " VIDE" } else { "" };
        ctx.logger.log(id, &format!("{head}{tag}"), &held);
        return if retry { None } else { Some(build(full(Bytes::from(held)))) };
    }

    let (mut tx, ch) = Channel::<Bytes, Infallible>::new(16);
    let logger = ctx.logger.clone();
    tokio::spawn(async move {
        let max = logger.max_body();
        let mut logbuf = held.clone();
        let _ = tx.send_data(Bytes::from(held)).await;
        while let Some(Ok(frame)) = body.frame().await {
            if let Ok(d) = frame.into_data() {
                if logbuf.len() < max {
                    logbuf.extend_from_slice(&d);
                }
                if tx.send_data(d).await.is_err() {
                    break;
                }
            }
        }
        logger.log(id, &head, &logbuf);
    });
    Some(build(ch.boxed()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_replacements_change_incoming_body_only() {
        let rules = vec![
            RequestReplacement {
                from: "SECRET".into(),
                to: "[redacted]".into(),
                first_only: true,
                from_end: false,
            },
            RequestReplacement {
                from: "REMOVE_ME".into(),
                to: "".into(),
                first_only: false,
                from_end: false,
            },
        ];

        let original = Bytes::from_static(b"PREFIX SECRET MIDDLE REMOVE_ME END SECRET");
        let rewritten = apply_request_replacements(&original, &rules);

        assert_eq!(String::from_utf8_lossy(&rewritten), "PREFIX [redacted] MIDDLE  END SECRET");
    }

    #[test]
    fn request_replacement_can_limit_to_first_occurrence() {
        let rules = vec![RequestReplacement {
            from: "ABC".into(),
            to: "X".into(),
            first_only: true,
            from_end: false,
        }];

        let rewritten = apply_request_replacements(b"ABC ABC ABC", &rules);
        assert_eq!(String::from_utf8_lossy(&rewritten), "X ABC ABC");
    }

    #[test]
    fn request_replacement_can_target_last_occurrence() {
        let rules = vec![RequestReplacement {
            from: "ABC".into(),
            to: "X".into(),
            first_only: true,
            from_end: true,
        }];

        let rewritten = apply_request_replacements(b"ABC ABC ABC", &rules);
        assert_eq!(String::from_utf8_lossy(&rewritten), "ABC ABC X");
    }

    #[test]
    fn request_replacements_report_when_prompt_changes() {
        let rules = vec![RequestReplacement {
            from: "SECRET".into(),
            to: "[redacted]".into(),
            first_only: false,
            from_end: false,
        }];

        let (rewritten, changed) = apply_request_replacements_with_change(b"SECRET prompt", &rules);
        assert!(changed);
        assert_eq!(String::from_utf8_lossy(&rewritten), "[redacted] prompt");
    }
}
