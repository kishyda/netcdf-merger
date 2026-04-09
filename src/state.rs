use std::collections::HashMap;

use rocket::futures::lock::Mutex;

#[derive(Default)]
pub struct AppState {
    pub part_a_files: Mutex<HashMap<String, Vec<u8>>>,
    pub part_b_files: Mutex<HashMap<String, Vec<u8>>>,
}
