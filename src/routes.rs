use rocket::Data;
use rocket::State as RocketState;
use rocket::data::ToByteUnit;
use rocket::http::ContentType;
use rocket::http::Status;

use crate::helpers;
use crate::state::{AppState, DataSet};

#[post("/part_a?<name>", format = "application/netcdf", data = "<data>")]
pub async fn part_a(
    name: &str,
    data: Data<'_>,
    state: &RocketState<AppState>,
) -> helpers::ApiResult<(Status, &'static str)> {
    validate_dataset_name(name)?;

    let stream = data.open(1.gigabytes());
    let bytes = stream.into_bytes().await?.into_inner();

    netcdf::open_mem(None, &bytes).map_err(|err| {
        helpers::ApiError::bad_request(format!("Body contains invalid netcdf data: {err}"))
    })?;

    let mut dataset = state
        .data
        .entry(name.to_string())
        .or_insert_with(DataSet::default);
    dataset.part_a_data = Some(bytes);
    dataset.merged_data = None;
    dataset.version = dataset.version.saturating_add(1);

    Ok((Status::Ok, "Success"))
}

#[post("/part_b?<name>", format = "application/netcdf", data = "<data>")]
pub async fn part_b(
    name: &str,
    data: Data<'_>,
    state: &RocketState<AppState>,
) -> helpers::ApiResult<(Status, &'static str)> {
    validate_dataset_name(name)?;

    let stream = data.open(1.gigabytes());
    let bytes = stream.into_bytes().await?.into_inner();

    netcdf::open_mem(None, &bytes).map_err(|err| {
        helpers::ApiError::bad_request(format!("Body contains invalid netcdf data: {err}"))
    })?;

    let mut dataset = state
        .data
        .entry(name.to_string())
        .or_insert_with(DataSet::default);
    dataset.part_b_data = Some(bytes);
    dataset.merged_data = None;
    dataset.version = dataset.version.saturating_add(1);

    Ok((Status::Ok, "Success"))
}

#[get("/read?<name>")]
pub async fn read_file(
    name: &str,
    state: &RocketState<AppState>,
) -> helpers::ApiResult<(ContentType, Vec<u8>)> {
    validate_dataset_name(name)?;

    loop {
        let Some(dataset) = state.data.get(name).map(|entry| entry.clone()) else {
            return Err(helpers::ApiError::not_found(format!(
                "no uploaded datasets found for name={name:?}"
            )));
        };

        if let Some(bytes) = dataset.merged_data {
            return Ok((ContentType::new("application", "netcdf"), bytes));
        }

        let part_a = dataset.part_a_data;
        let part_b = dataset.part_b_data;
        let version = dataset.version;

        let bytes = match (part_a.as_deref(), part_b.as_deref()) {
            (Some(part_a), Some(part_b)) => {
                crate::netcdf_operations::combine_netcdf_bytes(&[part_a, part_b])?
            }
            (None, None) => {
                return Err(helpers::ApiError::not_found(format!(
                    "no uploaded datasets found for name={name:?}"
                )));
            }
            (None, Some(_)) => {
                return Err(helpers::ApiError::conflict(format!(
                    "cannot read merged dataset for name={name:?}: missing part_a upload"
                )));
            }
            (Some(_), None) => {
                return Err(helpers::ApiError::conflict(format!(
                    "cannot read merged dataset for name={name:?}: missing part_b upload"
                )));
            }
        };

        let Some(mut dataset) = state.data.get_mut(name) else {
            return Err(helpers::ApiError::not_found(format!(
                "no uploaded datasets found for name={name:?}"
            )));
        };

        if dataset.version != version {
            continue;
        }

        if let Some(cached) = dataset.merged_data.clone() {
            return Ok((ContentType::new("application", "netcdf"), cached));
        }

        dataset.merged_data = Some(bytes.clone());
        return Ok((ContentType::new("application", "netcdf"), bytes));
    }
}

fn validate_dataset_name(name: &str) -> helpers::ApiResult<()> {
    if name.trim().is_empty() {
        return Err(helpers::ApiError::bad_request(
            "query parameter 'name' must not be empty",
        ));
    }

    Ok(())
}
