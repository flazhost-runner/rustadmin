//! `access` module — User / Role / Permission (RBAC). Reference module for the full
//! pattern: services (trait + impl), validators, web+api controllers, named method-aware
//! routes, and canonical index tables.

pub mod context;
pub mod controllers;
pub mod models;
pub mod routes;
pub mod services;
pub mod validators;
