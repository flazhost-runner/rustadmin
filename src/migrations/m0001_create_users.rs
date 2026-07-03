//! `users` table (canonical schema). `id` = varchar(36) UUID string (not auto-inc).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("users"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("code"))
                            .string_len(20)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Alias::new("name")).string_len(50).not_null())
                    .col(ColumnDef::new(Alias::new("phone")).string_len(15).null())
                    .col(
                        ColumnDef::new(Alias::new("email"))
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("email_verified_at"))
                            .timestamp()
                            .null(),
                    )
                    .col(ColumnDef::new(Alias::new("password")).string().not_null())
                    .col(ColumnDef::new(Alias::new("password_otp")).string().null())
                    .col(
                        ColumnDef::new(Alias::new("password_otp_expires"))
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("status"))
                            .string_len(20)
                            .not_null()
                            .default("Active"),
                    )
                    .col(ColumnDef::new(Alias::new("picture")).string().null())
                    .col(
                        ColumnDef::new(Alias::new("blocked"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Alias::new("blocked_reason")).string().null())
                    .col(
                        ColumnDef::new(Alias::new("timezone"))
                            .string()
                            .null()
                            .default("UTC"),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_by"))
                            .string_len(36)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("updated_by"))
                            .string_len(36)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Alias::new("updated_at"))
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("users__status")
                    .table(Alias::new("users"))
                    .col(Alias::new("status"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("users")).to_owned())
            .await
    }
}
