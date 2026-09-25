use std::collections::HashSet;

use super::{Db, DbError, found, place};
use crate::model::FolderId;
use crate::slugs::{slugify, unique_slug};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Folder {
    #[serde(skip)]
    pub id: FolderId,
    pub slug: String,
    pub name: String,
}

impl Db {
    pub async fn create_folder(&self, name: &str) -> Result<Folder, DbError> {
        let slug = self.free_folder_slug(name, None).await?;
        let id = sqlx::query_scalar!(
            r#"INSERT INTO folders (name, slug, position)
               VALUES (?, ?, (SELECT COALESCE(MAX(position), -1) + 1 FROM folders))
               RETURNING id AS "id: FolderId""#,
            name,
            slug
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Folder {
            id,
            slug,
            name: name.to_string(),
        })
    }

    /// Returns the folder with this name, creating it at the end if it doesn't exist.
    pub async fn ensure_folder(&self, name: &str) -> Result<Folder, DbError> {
        let existing = sqlx::query_as!(
            Folder,
            r#"SELECT id AS "id: FolderId", slug, name FROM folders WHERE name = ?"#,
            name
        )
        .fetch_optional(&self.pool)
        .await?;
        match existing {
            Some(folder) => Ok(folder),
            None => self.create_folder(name).await,
        }
    }

    /// Returns the new slug: a folder's URL follows its name.
    pub async fn rename_folder(&self, id: FolderId, name: &str) -> Result<String, DbError> {
        let slug = self.free_folder_slug(name, Some(id)).await?;
        found(
            sqlx::query!(
                "UPDATE folders SET name = ?, slug = ? WHERE id = ?",
                name,
                slug,
                id
            )
            .execute(&self.pool)
            .await?,
        )?;
        Ok(slug)
    }

    pub async fn folder_id(&self, slug: &str) -> Result<FolderId, DbError> {
        sqlx::query_scalar!(
            r#"SELECT id AS "id: FolderId" FROM folders WHERE slug = ?"#,
            slug
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DbError::NotFound)
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
            r#"SELECT id AS "id: FolderId", slug, name FROM folders ORDER BY position, id"#
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// A slug for `name` that no other folder uses (`except` is the folder being renamed).
    async fn free_folder_slug(
        &self,
        name: &str,
        except: Option<FolderId>,
    ) -> Result<String, DbError> {
        let base = slugify(name, "folder");
        let taken: HashSet<String> = sqlx::query_scalar!(
            "SELECT slug FROM folders WHERE (slug = ?1 OR slug LIKE ?1 || '-%') AND id IS NOT ?2",
            base,
            except
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect();
        Ok(unique_slug(&base, |slug| taken.contains(slug)))
    }
}
