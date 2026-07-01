//! Passive API-layer route contract helpers.
//!
//! These modules validate request and response shapes for future server wiring.
//! They do not register Axum routes, authenticate adapters, access storage, or
//! call Core services.

pub mod changes;
