mod api;
mod app_settings;
mod auth;
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
mod static_files;
mod tokens;
mod users;
mod sortkey;

use std::error::Error;

use topcoat::{
    cookie::RouterBuilderCookieExt,
    router::{Router, RouterBuilderDiscoverExt},
    session::{RouterBuilderSessionExt, SessionConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::var("FUGANTT_DB").unwrap_or_else(|_| "fugantt.db".to_owned());
    let pool = db::connect(&path).await?;

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

    topcoat::start(router).await?;

    Ok(())
}
