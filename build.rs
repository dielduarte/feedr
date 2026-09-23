use std::path::PathBuf;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};

// The sqlx query macros check SQL against a live database while compiling. Rebuilding a
// throwaway one from the migrations keeps those checks in sync with the schema, with no setup.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=migrations");

    let path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("schema.db");
    let _ = std::fs::remove_file(&path);

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let options = SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true);
            let pool = SqlitePool::connect_with(options).await.unwrap();
            Migrator::new(PathBuf::from("migrations"))
                .await
                .unwrap()
                .run(&pool)
                .await
                .unwrap();
            pool.close().await;
        });

    println!("cargo:rustc-env=DATABASE_URL=sqlite:{}", path.display());
}
