use super::{Db, DbError, found, place};
use crate::model::FolderId;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Folder {
    pub id: FolderId,
    pub name: String,
}

impl Db {
    pub async fn create_folder(&self, name: &str) -> Result<Folder, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO folders (name, position)
             VALUES (?, (SELECT COALESCE(MAX(position), -1) + 1 FROM folders))
             RETURNING id",
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await?;
        Ok(Folder {
            id,
            name: name.to_string(),
        })
    }

    pub async fn rename_folder(&self, id: FolderId, name: &str) -> Result<(), DbError> {
        found(
            sqlx::query("UPDATE folders SET name = ? WHERE id = ?")
                .bind(name)
                .bind(id)
                .execute(&self.pool)
                .await?,
        )
    }

    pub async fn move_folder(&self, id: FolderId, index: usize) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;
        let siblings: Vec<FolderId> =
            sqlx::query_scalar("SELECT id FROM folders ORDER BY position, id")
                .fetch_all(&mut *tx)
                .await?;
        if !siblings.contains(&id) {
            return Err(DbError::NotFound);
        }
        for (position, folder) in place(siblings, id, index).into_iter().enumerate() {
            sqlx::query("UPDATE folders SET position = ? WHERE id = ?")
                .bind(position as i64)
                .bind(folder)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_folder(&self, id: FolderId) -> Result<(), DbError> {
        found(
            sqlx::query("DELETE FROM folders WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?,
        )
    }

    pub(super) async fn folders(&self) -> Result<Vec<Folder>, DbError> {
        Ok(
            sqlx::query_as("SELECT id, name FROM folders ORDER BY position, id")
                .fetch_all(&self.pool)
                .await?,
        )
    }
}
