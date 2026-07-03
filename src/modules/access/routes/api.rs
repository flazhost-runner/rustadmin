//! API routes for the access module (mounted at `/api/v1`).

use rocket::Route;

use crate::modules::access::controllers::api;

pub fn routes() -> Vec<Route> {
    routes![
        // user
        api::user::index,
        api::user::store,
        api::user::edit,
        api::user::update,
        api::user::delete,
        api::user::delete_selected,
        // role
        api::role::index,
        api::role::store,
        api::role::edit,
        api::role::update,
        api::role::delete,
        api::role::delete_selected,
        api::role::permission,
        api::role::assign,
        api::role::unassign,
        api::role::assign_selected,
        api::role::unassign_selected,
        // permission
        api::permission::index,
        api::permission::store,
        api::permission::edit,
        api::permission::update,
        api::permission::delete,
        api::permission::delete_selected,
    ]
}
