//! `users_roles(user_id, role_id)` join table — composite PK (pinned name + columns).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("users_roles"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("user_id"))
                            .string_len(36)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("role_id"))
                            .string_len(36)
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(Alias::new("user_id"))
                            .col(Alias::new("role_id")),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("users_roles")).to_owned())
            .await
    }
}
