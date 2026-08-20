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
mod notfound;
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
        Some("--make-admin") => {
            let who = std::env::args().nth(2);

            return make_admin(who);
        }
        _ => {}
    }

    serve(settings)
}

/// Hands the administrator's role to an account, from the machine itself.
///
/// The screen refuses to remove the last administrator, which stops the plan
/// being locked by a slip of the mouse. It cannot help with the other way in:
/// the one administrator forgets their password, or leaves. There is no mail
/// to send a reset with — deliberately — so the way back has to be the machine
/// the data is on.
///
/// Whoever can run this can already read the database file, so this grants
/// nothing that was not already theirs. It only saves them writing the SQL.
#[tokio::main]
async fn make_admin(who: Option<String>) -> Result<(), Box<dyn Error>> {
    let Some(who) = who
        .map(|who| who.trim().to_owned())
        .filter(|who| !who.is_empty())
    else {
        eprintln!("使い方: fugantt --make-admin <ユーザー名>");
        std::process::exit(1);
    };

    let path = db::path();
    let pool = db::connect(&path.to_string_lossy()).await?;

    let changed = sqlx::query("UPDATE users SET base_role = 'admin' WHERE email = ?1")
        .bind(&who)
        .execute(&pool)
        .await?
        .rows_affected();

    if changed == 0 {
        eprintln!("{who} という利用者は居ません。");
        eprintln!("データ: {}", path.display());
        std::process::exit(1);
    }

    println!("{who} を管理者にしました。");

    Ok(())
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
        .layer(notfound::NotFound)
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

    if let Err(error) = topcoat::start(router).await {
        // The one failure that is somebody else's fault and looks like ours: a
        // raw "Address already in use" reads as a crash to whoever just
        // double-clicked this.
        if error.kind() == std::io::ErrorKind::AddrInUse {
            eprintln!("{port} は他のプログラムが使っています。");
            eprintln!("別の番号にするには、fugantt.ini に PORT = 1862 のように書くか、");
            eprintln!("PORT=1862 を渡してください。");
            std::process::exit(1);
        }

        return Err(error.into());
    }

    Ok(())
}
