mod api;
mod app_settings;
mod auth;
mod backup;
mod browser;
mod config;
mod db;
mod domain;
mod history;
mod holidays;
mod i18n;
mod interop;
mod live;
mod open_access;
mod pages;
mod project;
mod ratelimit;
mod session_cookie;
mod sortkey;
mod static_files;
mod tokens;
mod users;

use std::error::Error;

use topcoat::{
    cookie::RouterBuilderCookieExt,
    router::{Router, RouterBuilderDiscoverExt},
    session::{RouterBuilderSessionExt, SessionConfig},
};

fn main() -> Result<(), Box<dyn Error>> {
    // Before the runtime, because this writes to the process environment and
    // that is only sound while the process is still one thread.
    //
    // SAFETY: nothing else has started.
    let settings = unsafe { config::load() };

    match std::env::args().nth(1).as_deref() {
        Some("--help" | "-h" | "help") => {
            print!("{}", config::help());
            return Ok(());
        }
        Some("--config") => {
            print!("{}", config::explain(&settings));
            return Ok(());
        }
        Some("--version" | "-V") => {
            println!("fugantt {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    serve(settings)
}

#[tokio::main]
async fn serve(settings: config::Loaded) -> Result<(), Box<dyn Error>> {
    if let Some(file) = &settings.path {
        println!("設定: {}", file.display());
    }

    let path = db::path();
    db::remember(&path);
    let pool = db::connect(&path.to_string_lossy()).await?;

    // Said out loud, and as an absolute path. The default moved to a per-user
    // directory, and "which file am I looking at" must never be a question a
    // person has to answer by guessing where they happened to be standing.
    println!(
        "データ: {}",
        std::fs::canonicalize(&path)
            .unwrap_or_else(|_| path.clone())
            .display()
    );

    match open_access::check() {
        Ok(Some(warning)) => {
            eprintln!("警告: {warning}");
            eprintln!("      信頼できるネットワークでのみ使ってください。");
        }
        Ok(None) => {}
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }

    if session_cookie::allows_http() {
        // Loud on purpose: this is the one setting that trades a real defence
        // for reachability, and nobody should discover it by reading the code.
        eprintln!("警告: FUGANTT_ALLOW_HTTP=1 のため、セッションクッキーを平文で送ります。");
        eprintln!("      信頼できるネットワークでのみ使ってください。");
    }

    let router = Router::builder()
        .cookies()
        .sessions(
            SessionConfig::builder()
                .token_store(session_cookie::SessionCookie::new())
                .build(),
        )
        .app_context(db::Db(pool))
        .app_context(live::Hub::default())
        .app_context(ratelimit::Attempts::default())
        // Pages, layouts, and routes register themselves at link time.
        .discover()
        .build();

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(3000);

    println!("画面: {}", browser::url(&host, port));

    // The page opens itself once the port answers. The alternative is a console
    // window and the expectation that a person knows the point of it is a URL.
    let open = browser::wanted(&host);
    tokio::spawn(async move { browser::open_when_ready(open, &host, port).await });

    topcoat::start(router).await?;

    Ok(())
}
