pub mod helpers;
pub mod server;
pub mod state;
pub mod worker;

pub use server::server::build_rocket;
pub use worker::netcdf_operations;
