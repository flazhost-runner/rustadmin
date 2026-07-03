//! Permission form DTO + validation.

use rocket::form::FromForm;
use serde_json::{json, Value};

use crate::modules::access::services::permission_service::PermissionInput;
use crate::modules::access::validators::FormError;

#[derive(FromForm, Debug, Default)]
pub struct PermissionForm {
    pub name: Option<String>,
    pub guard_name: Option<String>,
    pub method: Option<String>,
    pub status: Option<String>,
    pub desc: Option<String>,
}

impl PermissionForm {
    fn old(&self) -> Value {
        json!({
            "name": t(&self.name),
            "guard_name": t(&self.guard_name),
            "method": t(&self.method),
            "status": t(&self.status),
            "desc": t(&self.desc),
        })
    }

    pub fn validate(self) -> Result<PermissionInput, FormError> {
        if t(&self.name).is_empty() {
            return Err(FormError {
                errors: json!({ "name": "Name is required" }),
                old: self.old(),
            });
        }
        Ok(PermissionInput {
            name: t(&self.name),
            guard_name: o(&self.guard_name),
            method: o(&self.method),
            status: o(&self.status),
            desc: o(&self.desc),
        })
    }
}

fn t(o: &Option<String>) -> String {
    o.as_deref().unwrap_or("").trim().to_string()
}
fn o(v: &Option<String>) -> Option<String> {
    let s = t(v);
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
