#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, serde::Serialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct FolderId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, serde::Serialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct FeedId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, serde::Serialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct ItemId(pub i64);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Validators {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedScope {
    All,
    Folder(FolderId),
    Feed(FeedId),
}
