//! `auth` module — login/logout (web session + API JWT), registration, password reset.
//! Phase 3 ships the API auth + service; web views/flows land in later phases.

pub mod routes;
pub mod service;
