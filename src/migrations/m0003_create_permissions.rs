//! `permissions` table. `name` is **indexed but NON-unique** (multiple methods/guards may
//! share a name). `method` + `guard_name` complete the route-driven permission tuple.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("permissions"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Alias::new("name")).string().not_null())
                    .col(
                        ColumnDef::new(Alias::new("guard_name"))
                            .string_len(20)
                            .not_null()
                            .default("web"),
                    )
                    .col(ColumnDef::new(Alias::new("method")).string().null())
                    .col(
                        ColumnDef::new(Alias::new("status"))
                            .string_len(20)
                            .not_null()
                            .default("Active"),
                    )
                    .col(ColumnDef::new(Alias::new("desc")).string().null())
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
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Alias::new("updated_at"))
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // non-unique index on name + an index on guard_name (filtered columns)
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("permissions__name")
                    .table(Alias::new("permissions"))
                    .col(Alias::new("name"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("permissions__guard")
                    .table(Alias::new("permissions"))
                    .col(Alias::new("guard_name"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("permissions")).to_owned())
            .await
    }
}
