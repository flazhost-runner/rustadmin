//! Web routes for the access module (mounted at `/admin/v1`).

use rocket::Route;

use crate::modules::access::controllers::web;

pub fn routes() -> Vec<Route> {
    routes![
        // user
        web::user::index,
        web::user::create,
        web::user::store,
        web::user::edit,
        web::user::update,
        web::user::delete,
        web::user::delete_selected,
        // role
        web::role::index,
        web::role::create,
        web::role::store,
        web::role::edit,
        web::role::update,
        web::role::delete,
        web::role::delete_selected,
        web::role::permission,
        web::role::assign,
        web::role::unassign,
        web::role::assign_selected,
        web::role::unassign_selected,
        // permission
        web::permission::index,
        web::permission::create,
        web::permission::store,
        web::permission::edit,
        web::permission::update,
        web::permission::delete,
        web::permission::delete_selected,
    ]
}
