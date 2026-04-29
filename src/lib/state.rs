use dashmap::DashMap;
use rocket::futures::lock::Mutex;
use crate::worker::pool::WorkerPool;

#[derive(Default)]
pub struct AppState {
    pub data: DashMap<String, DataSet>,
    pub netcdf_lock: Mutex<()>,
    pub process_pool: WorkerPool,
}

#[derive(Default, Clone)]
pub struct DataSet {
    pub part_a_data: Option<Vec<u8>>,
    pub part_b_data: Option<Vec<u8>>,
    pub merged_data: Option<Vec<u8>>,
    pub version: u64,
}
