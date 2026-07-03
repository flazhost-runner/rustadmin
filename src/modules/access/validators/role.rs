//! Role form DTO + validation.

use rocket::form::FromForm;
use serde_json::{json, Value};

use crate::modules::access::services::role_service::RoleInput;
use crate::modules::access::validators::FormError;

#[derive(FromForm, Debug, Default)]
pub struct RoleForm {
    pub name: Option<String>,
    pub status: Option<String>,
    pub desc: Option<String>,
}

impl RoleForm {
    fn old(&self) -> Value {
        json!({
            "name": t(&self.name),
            "status": t(&self.status),
            "desc": t(&self.desc),
        })
    }

    pub fn validate(self) -> Result<RoleInput, FormError> {
        if t(&self.name).is_empty() {
            return Err(FormError {
                errors: json!({ "name": "Name is required" }),
                old: self.old(),
            });
        }
        Ok(RoleInput {
            name: t(&self.name),
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
