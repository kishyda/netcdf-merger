use crate::server::routes;
use crate::state;

pub fn build_rocket() -> rocket::Rocket<rocket::Build> {
    println!("Process started at {}", std::process::id());

    rocket::build()
        .manage(state::AppState::default())
        .mount("/", rocket::routes![routes::read_file, routes::part_a, routes::part_b])
}

pub async fn run_server() -> Result<(), rocket::Error> {
    build_rocket()
        .launch()
        .await?;
    Ok(())
}
