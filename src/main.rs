#[macro_use] extern crate rocket;

struct State {

}

#[post("/part_a")]
fn part_a() -> String {
    format!("POST /part_a")
}

#[post("/part_b")]
fn part_b() -> String {
    format!("POST /part_b")
}

#[get("/read")]
fn read_file() -> String {
    format!("GET /read")
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(State{})
        .mount("/", routes![read_file, part_a, part_b])
}
