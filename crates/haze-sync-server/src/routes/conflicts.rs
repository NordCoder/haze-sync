//! Route composition wrapper preserving the accepted conflict implementation.

#[path = "conflicts_impl.rs"]
mod original;
#[path = "gdrive.rs"]
mod gdrive;

pub fn router() -> axum::Router {
    original::router().merge(gdrive::router())
}
