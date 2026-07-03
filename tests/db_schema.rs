//! Phase 2 — canonical schema migrates on SQLite in-memory, seeds the admin, and the
//! tricky bits hold: `desc` reserved word round-trips, `permissions.name` is non-unique,
//! and `ci_like` works case-insensitively across the dialect.

use rust_admin::db;
use rust_admin::helpers::ci_like;
use rust_admin::migrations::Migrator;
use rust_admin::modules::access::models::{permission, role, user};

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use sea_orm_migration::MigratorTrait;
use uuid::Uuid;

async fn fresh_db() -> sea_orm::DatabaseConnection {
    let conn = db::connect_in_memory().await.expect("connect in-memory");
    Migrator::up(&conn, None).await.expect("migrate up");
    conn
}

#[tokio::test]
async fn migrates_and_seeds_admin() {
    let conn = fresh_db().await;

    let admin = user::Entity::find()
        .filter(user::Column::Email.eq("admin@admin.com"))
        .one(&conn)
        .await
        .unwrap();
    let admin = admin.expect("admin user seeded");
    assert_eq!(admin.code, "0000000001");
    assert_eq!(admin.status, "Active");
    assert!(!admin.blocked);

    let admin_role = role::Entity::find()
        .filter(role::Column::Name.eq("Administrator"))
        .one(&conn)
        .await
        .unwrap();
    let admin_role = admin_role.expect("Administrator role seeded");
    assert_eq!(admin_role.status, "Active");
}

#[tokio::test]
async fn desc_reserved_word_roundtrips() {
    let conn = fresh_db().await;

    let new = role::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        name: Set("Editor".to_string()),
        status: Set("Active".to_string()),
        desc: Set(Some("Can edit content".to_string())),
        ..Default::default()
    };
    new.insert(&conn).await.expect("insert role with desc");

    let fetched = role::Entity::find()
        .filter(role::Column::Name.eq("Editor"))
        .one(&conn)
        .await
        .unwrap()
        .expect("editor role");
    assert_eq!(fetched.desc.as_deref(), Some("Can edit content"));
}

#[tokio::test]
async fn permission_name_is_non_unique() {
    let conn = fresh_db().await;

    for method in ["GET", "DELETE"] {
        let p = permission::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            name: Set("admin.v1.access.user.index".to_string()),
            guard_name: Set("web".to_string()),
            method: Set(Some(method.to_string())),
            status: Set("Active".to_string()),
            desc: Set(None),
            ..Default::default()
        };
        p.insert(&conn)
            .await
            .expect("insert permission (name reused)");
    }

    let count = permission::Entity::find()
        .filter(permission::Column::Name.eq("admin.v1.access.user.index"))
        .count(&conn)
        .await
        .unwrap();
    assert_eq!(count, 2, "permissions.name must allow duplicates");
}

#[tokio::test]
async fn ci_like_is_case_insensitive() {
    let conn = fresh_db().await;

    // seeded admin name is "Administrator" — search lower-case should still match
    let found = user::Entity::find()
        .filter(ci_like(user::Column::Name, "ADMIN"))
        .all(&conn)
        .await
        .unwrap();
    assert!(
        found.iter().any(|u| u.name == "Administrator"),
        "ci_like should match regardless of case"
    );
}
