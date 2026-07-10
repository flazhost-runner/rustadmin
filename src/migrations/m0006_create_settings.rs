//! `settings` singleton table. `description` = text; `theme` default Blue; `fe_template`
//! default = the pinned opentailwind slug.

use sea_orm_migration::prelude::*;

use crate::config::fe_templates::DEFAULT_FE_TEMPLATE;
use crate::config::themes::DEFAULT_THEME;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let string255 = |name: &str| ColumnDef::new(Alias::new(name)).string().null().to_owned();

        manager
            .create_table(
                Table::create()
                    .table(Alias::new("settings"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(string255("initial"))
                    .col(string255("name"))
                    .col(ColumnDef::new(Alias::new("description")).text().null())
                    .col(string255("icon"))
                    .col(string255("logo"))
                    .col(string255("login_image"))
                    .col(string255("phone"))
                    .col(string255("address"))
                    .col(string255("email"))
                    .col(string255("copyright"))
                    .col(
                        ColumnDef::new(Alias::new("theme"))
                            .string_len(20)
                            .null()
                            .default(DEFAULT_THEME),
                    )
                    .col(
                        ColumnDef::new(Alias::new("fe_template"))
                            .string_len(80)
                            .null()
                            .default(DEFAULT_FE_TEMPLATE),
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
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Alias::new("settings")).to_owned())
            .await
    }
}
