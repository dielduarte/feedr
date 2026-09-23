use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::path::PathBuf;

use anyhow::Context;
use chrono::Utc;
use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use tokio::sync::broadcast::{self, error::RecvError};
use tokio_util::sync::CancellationToken;

use crate::add_feed::{add_feed, parse_input};
use crate::api::{self, AppState};
use crate::db::{Db, SidebarFeed};
use crate::fetch::{DEFAULT_TIMEOUT, Fetcher};
use crate::model::FeedScope;
use crate::opml;
use crate::poller::{self, BatchHealth, PollerEvent, run_batch};

#[derive(Parser)]
#[command(name = "feedr", version, about = "A local-first RSS reader")]
pub struct Cli {
    /// Database file [default: the platform data directory]
    #[arg(long, env = "FEEDR_DB", global = true)]
    pub db: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run the background poller and the web API
    Serve {
        #[arg(long, default_value_t = 7777)]
        port: u16,
        /// Open the web app in your browser
        #[arg(long)]
        open: bool,
    },
    /// Subscribe to a feed, or to a site that links to one
    Add {
        url: String,
        /// Put the feed in this folder, creating it if needed
        #[arg(long)]
        folder: Option<String>,
    },
    /// Fetch every feed now
    Refresh,
    /// List folders and feeds with unread counts
    Ls,
    /// Import subscriptions from an OPML file
    Import { file: PathBuf },
    /// Print subscriptions as OPML
    Export,
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    let path = database_path(cli.db)?;
    let db = Db::open(&path)
        .await
        .with_context(|| format!("could not open {}", path.display()))?;
    match cli.command {
        Command::Serve { port, open } => serve(db, port, open).await,
        Command::Add { url, folder } => add(db, &url, folder).await,
        Command::Refresh => refresh(db).await,
        Command::Ls => list(db).await,
        Command::Import { file } => import(db, file).await,
        Command::Export => {
            print!("{}", opml::render(&db.sidebar().await?));
            Ok(())
        }
    }
}

fn database_path(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let path = match explicit {
        Some(path) => path,
        None => ProjectDirs::from("", "", "feedr")
            .context("could not find a data directory; pass --db")?
            .data_dir()
            .join("feedr.db"),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(path)
}

fn plural(count: impl Into<u64>, word: &str) -> String {
    match count.into() {
        1 => format!("1 {word}"),
        n => format!("{n} {word}s"),
    }
}

async fn serve(db: Db, port: u16, open: bool) -> anyhow::Result<()> {
    let fetcher = Fetcher::new(DEFAULT_TIMEOUT);
    let cancel = CancellationToken::new();
    let (poller, poller_task) = poller::spawn(db.clone(), fetcher.clone(), cancel.clone());
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port))
        .await
        .with_context(|| format!("could not listen on port {port}"))?;
    let url = format!("http://{}", listener.local_addr()?);
    println!("feedr is running at {url}");
    if open && let Err(error) = open::that_detached(&url) {
        tracing::warn!(%error, "could not open a browser");
    }

    let app = api::router(AppState {
        db,
        fetcher,
        poller,
    });
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            cancel.cancel();
        })
        .await?;
    poller_task.await?;
    Ok(())
}

async fn shutdown_signal() {
    let interrupt = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                terminate.recv().await;
            }
            Err(_) => std::future::pending().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = interrupt => {}
        () = terminate => {}
    }
}

async fn add(db: Db, input: &str, folder: Option<String>) -> anyhow::Result<()> {
    let url = parse_input(input).with_context(|| format!("not a web address: {input}"))?;
    let folder = match folder {
        Some(name) => Some(db.ensure_folder(&name).await?.id),
        None => None,
    };
    let fetcher = Fetcher::new(DEFAULT_TIMEOUT);
    let added = add_feed(&db, &fetcher, &url, folder, Utc::now()).await?;
    println!(
        "Added {} ({})",
        added.title,
        plural(added.new_items, "new item")
    );
    Ok(())
}

async fn refresh(db: Db) -> anyhow::Result<()> {
    let now = Utc::now();
    db.mark_due(FeedScope::All, now).await?;
    let feeds = db.feeds_due(now).await?;
    if feeds.is_empty() {
        println!("No feeds yet. Add one with `feedr add <url>`.");
        return Ok(());
    }
    let count = feeds.len() as u64;
    let titles: HashMap<_, _> = feeds
        .iter()
        .map(|f| {
            (
                f.id,
                f.custom_title.clone().unwrap_or_else(|| f.title.clone()),
            )
        })
        .collect();

    let (events, mut received) = broadcast::channel(1024);
    let printer = tokio::spawn(async move {
        let (mut new_items, mut failed) = (0, 0u64);
        loop {
            match received.recv().await {
                Ok(PollerEvent::FeedRefreshed { feed, new_items: n }) if n > 0 => {
                    new_items += n;
                    println!("  +{n:<4} {}", titles[&feed]);
                }
                Ok(PollerEvent::FeedFailed { feed, error }) => {
                    failed += 1;
                    println!("  !     {}: {error}", titles[&feed]);
                }
                Ok(_) | Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return (new_items, failed),
            }
        }
    });

    let health = run_batch(&db, &Fetcher::new(DEFAULT_TIMEOUT), feeds, now, &events).await;
    drop(events);
    let (new_items, failed) = printer.await?;

    match health {
        BatchHealth::Offline => println!("You seem to be offline; nothing was changed."),
        BatchHealth::Online => {
            let failures = match failed {
                0 => String::new(),
                n => format!(", {} failed", plural(n, "feed")),
            };
            println!(
                "Checked {}: {}{failures}",
                plural(count, "feed"),
                plural(new_items, "new item")
            );
        }
    }
    Ok(())
}

async fn list(db: Db) -> anyhow::Result<()> {
    let sidebar = db.sidebar().await?;
    if sidebar.folders.is_empty() && sidebar.uncategorized.is_empty() {
        println!("No feeds yet. Add one with `feedr add <url>`.");
        return Ok(());
    }
    for folder in &sidebar.folders {
        println!("{}", folder.folder.name);
        folder.feeds.iter().for_each(print_feed);
    }
    if !sidebar.uncategorized.is_empty() {
        println!("Uncategorized");
        sidebar.uncategorized.iter().for_each(print_feed);
    }
    println!("{} unread", sidebar.total_unread());
    Ok(())
}

fn print_feed(feed: &SidebarFeed) {
    match &feed.last_error {
        None => println!("{:>5}  {}", feed.unread, feed.title),
        Some(error) => println!("{:>5}  {}  (failing: {error})", feed.unread, feed.title),
    }
}

async fn import(db: Db, file: PathBuf) -> anyhow::Result<()> {
    let xml = std::fs::read_to_string(&file)
        .with_context(|| format!("could not read {}", file.display()))?;
    let report = opml::import(&db, opml::parse(&xml)?, Utc::now()).await?;
    println!(
        "Added {}, skipped {} already subscribed.",
        plural(report.added, "feed"),
        report.skipped
    );
    for url in &report.invalid {
        println!("  ignored invalid feed URL: {url}");
    }
    if report.added > 0 {
        println!("Run `feedr refresh` or `feedr serve` to fetch them.");
    }
    Ok(())
}
