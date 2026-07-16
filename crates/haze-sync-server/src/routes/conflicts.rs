//! Route composition wrapper preserving the accepted conflict implementation.

#[path = "gdrive.rs"]
mod gdrive;
#[path = "conflicts_impl.rs"]
mod original;

pub fn router() -> axum::Router {
    original::router().merge(gdrive::router())
}

#[cfg(test)]
#[path = "gdrive_tests.rs"]
mod gdrive_tests;
