use super::{Db, DbError, found, place};
use crate::model::FolderId;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Folder {
    pub id: FolderId,
    pub name: String,
}

impl Db {
    pub async fn create_folder(&self, name: &str) -> Result<Folder, DbError> {
        let id = sqlx::query_scalar!(
            r#"INSERT INTO folders (name, position)
               VALUES (?, (SELECT COALESCE(MAX(position), -1) + 1 FROM folders))
               RETURNING id AS "id: FolderId""#,
            name
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Folder {
            id,
            name: name.to_string(),
        })
    }

    /// Returns the folder with this name, creating it at the end if it doesn't exist.
    pub async fn ensure_folder(&self, name: &str) -> Result<Folder, DbError> {
        // The no-op update makes RETURNING yield the existing row on conflict.
        let id = sqlx::query_scalar!(
            r#"INSERT INTO folders (name, position)
               VALUES (?, (SELECT COALESCE(MAX(position), -1) + 1 FROM folders))
               ON CONFLICT (name) DO UPDATE SET name = excluded.name
               RETURNING id AS "id: FolderId""#,
            name
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Folder {
            id,
            name: name.to_string(),
        })
    }

    pub async fn rename_folder(&self, id: FolderId, name: &str) -> Result<(), DbError> {
        found(
            sqlx::query!("UPDATE folders SET name = ? WHERE id = ?", name, id)
                .execute(&self.pool)
                .await?,
        )
    }

    pub async fn move_folder(&self, id: FolderId, index: usize) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;
        let siblings = sqlx::query_scalar!(
            r#"SELECT id AS "id: FolderId" FROM folders ORDER BY position, id"#
        )
        .fetch_all(&mut *tx)
        .await?;
        if !siblings.contains(&id) {
            return Err(DbError::NotFound);
        }
        for (position, folder) in place(siblings, id, index).into_iter().enumerate() {
            let position = position as i64;
            sqlx::query!(
                "UPDATE folders SET position = ? WHERE id = ?",
                position,
                folder
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_folder(&self, id: FolderId) -> Result<(), DbError> {
        found(
            sqlx::query!("DELETE FROM folders WHERE id = ?", id)
                .execute(&self.pool)
                .await?,
        )
    }

    pub(super) async fn folders(&self) -> Result<Vec<Folder>, DbError> {
        Ok(sqlx::query_as!(
            Folder,
            r#"SELECT id AS "id: FolderId", name FROM folders ORDER BY position, id"#
        )
        .fetch_all(&self.pool)
        .await?)
    }
}
