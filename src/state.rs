use dashmap::DashMap;
use rocket::futures::lock::Mutex;

#[derive(Default)]
pub struct AppState {
    pub data: DashMap<String, DataSet>,
    pub netcdf_lock: Mutex<()>,
}

#[derive(Default, Clone)]
pub struct DataSet {
    pub part_a_data: Option<Vec<u8>>,
    pub part_b_data: Option<Vec<u8>>,
    pub merged_data: Option<Vec<u8>>,
    pub version: u64,
}
