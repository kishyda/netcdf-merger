#[macro_use]
extern crate rocket;

pub mod helpers;
pub mod netcdf_operations;
pub mod routes;
pub mod state;

pub fn build_rocket() -> rocket::Rocket<rocket::Build> {
    rocket::build().manage(state::AppState::default()).mount(
        "/",
        routes![routes::read_file, routes::part_a, routes::part_b],
    )
}
