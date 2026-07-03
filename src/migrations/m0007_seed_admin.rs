//! Seed the Administrator role + admin user (linked) + the settings singleton.
//! Mirrors NodeAdmin `AddAdminUser`: admin@admin.com / 12345678, code 0000000001.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::SimpleExpr;
use uuid::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Idempotent: skip if admin@admin.com already exists.
        let already = manager
            .get_connection()
            .query_one(sea_orm::Statement::from_string(
                manager.get_database_backend(),
                "SELECT 1 FROM users WHERE email = 'admin@admin.com' LIMIT 1".to_owned(),
            ))
            .await?
            .is_some();
        if already {
            return Ok(());
        }

        let user_id = Uuid::new_v4().to_string();
        let role_id = Uuid::new_v4().to_string();
        let setting_id = Uuid::new_v4().to_string();
        let password =
            bcrypt::hash("12345678", 10).map_err(|e| DbErr::Custom(format!("bcrypt: {e}")))?;

        // Administrator role
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Alias::new("roles"))
                    .columns([
                        Alias::new("id"),
                        Alias::new("name"),
                        Alias::new("status"),
                        Alias::new("desc"),
                    ])
                    .values_panic([
                        role_id.clone().into(),
                        "Administrator".into(),
                        "Active".into(),
                        "".into(),
                    ])
                    .to_owned(),
            )
            .await?;

        // Admin user
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Alias::new("users"))
                    .columns([
                        Alias::new("id"),
                        Alias::new("code"),
                        Alias::new("name"),
                        Alias::new("phone"),
                        Alias::new("email"),
                        Alias::new("email_verified_at"),
                        Alias::new("password"),
                        Alias::new("status"),
                        Alias::new("timezone"),
                        Alias::new("blocked"),
                        Alias::new("blocked_reason"),
                    ])
                    .values_panic([
                        user_id.clone().into(),
                        "0000000001".into(),
                        "Administrator".into(),
                        "12345678910".into(),
                        "admin@admin.com".into(),
                        SimpleExpr::Custom("CURRENT_TIMESTAMP".to_owned()),
                        password.into(),
                        "Active".into(),
                        "Asia/Jakarta".into(),
                        false.into(),
                        "".into(),
                    ])
                    .to_owned(),
            )
            .await?;

        // Link user ↔ role
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Alias::new("users_roles"))
                    .columns([Alias::new("user_id"), Alias::new("role_id")])
                    .values_panic([user_id.into(), role_id.into()])
                    .to_owned(),
            )
            .await?;

        // Settings singleton (theme/fe_template fall back to column defaults)
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Alias::new("settings"))
                    .columns([Alias::new("id"), Alias::new("name")])
                    .values_panic([setting_id.into(), "RustAdmin".into()])
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove seeded rows (best-effort).
        for (table, col, val) in [
            ("users", "email", "admin@admin.com"),
            ("roles", "name", "Administrator"),
        ] {
            manager
                .exec_stmt(
                    Query::delete()
                        .from_table(Alias::new(table))
                        .and_where(Expr::col(Alias::new(col)).eq(val))
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
