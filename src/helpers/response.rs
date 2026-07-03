//! JSON response envelope for the API (mirrors NodeAdmin `ResponseHandler`).
//!
//! Shape: `{ success, message, data?, meta? }`. Errors go through [`crate::errors::AppError`]'s
//! `Responder`, so these helpers are for the success paths.

use rocket::http::Status;
use rocket::serde::json::{json, Json};
use serde::Serialize;
use serde_json::Value;

use crate::helpers::pagination::PaginationMeta;

/// `200 OK` with `data`.
pub fn success<T: Serialize>(data: T) -> (Status, Json<Value>) {
    (
        Status::Ok,
        Json(json!({ "status": true, "message": "OK", "data": data })),
    )
}

/// `200 OK` with a custom message and `data`.
pub fn success_msg<T: Serialize>(message: &str, data: T) -> (Status, Json<Value>) {
    (
        Status::Ok,
        Json(json!({ "status": true, "message": message, "data": data })),
    )
}

/// `201 Created` with `data`.
pub fn created<T: Serialize>(data: T) -> (Status, Json<Value>) {
    (
        Status::Created,
        Json(json!({ "status": true, "message": "Created", "data": data })),
    )
}

/// `200 OK` for a message-only response (e.g. delete).
pub fn message(message: &str) -> (Status, Json<Value>) {
    (
        Status::Ok,
        Json(json!({ "status": true, "message": message, "data": null })),
    )
}

/// `200 OK` for a paginated list.
pub fn paginated<T: Serialize>(data: T, meta: &PaginationMeta) -> (Status, Json<Value>) {
    (
        Status::Ok,
        Json(json!({ "status": true, "message": "OK", "data": data, "meta": meta })),
    )
}
