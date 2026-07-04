//! HTTP helpers for the Haze Sync server scaffold.
//!
//! These helpers define safe public response shapes for placeholder routes. They
//! intentionally do not expose stack traces, secrets, provider payloads, local
//! filesystem paths, or runtime internals.

pub mod errors;
