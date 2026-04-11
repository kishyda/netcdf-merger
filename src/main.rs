use rocket::launch;

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    println!("Process started at {}", std::process::id());
    windborne_oa::build_rocket()
}
