use rocket::launch;

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    windborne_oa::build_rocket()
}
