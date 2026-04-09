use rocket::Data;
use rocket::State as RocketState;
use rocket::data::ToByteUnit;
use rocket::http::ContentType;
use rocket::http::Status;

use crate::helpers;
use crate::state::AppState;

#[post("/part_a?<name>", format = "application/netcdf", data = "<data>")]
pub async fn part_a(
    name: &str,
    data: Data<'_>,
    state: &RocketState<AppState>,
) -> helpers::ApiResult<(Status, &'static str)> {
    validate_dataset_name(name)?;

    let stream = data.open(1.gigabytes());
    let bytes = stream.into_bytes().await?.into_inner();

    state
        .part_a_files
        .lock()
        .await
        .insert(name.to_string(), bytes);

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

    state
        .part_b_files
        .lock()
        .await
        .insert(name.to_string(), bytes);

    Ok((Status::Ok, "Success"))
}

#[get("/read?<name>")]
pub async fn read_file(
    name: &str,
    state: &RocketState<AppState>,
) -> helpers::ApiResult<(ContentType, Vec<u8>)> {
    validate_dataset_name(name)?;

    let part_a = state.part_a_files.lock().await.get(name).cloned();
    let part_b = state.part_b_files.lock().await.get(name).cloned();

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

    Ok((ContentType::new("application", "netcdf"), bytes))
}

fn validate_dataset_name(name: &str) -> helpers::ApiResult<()> {
    if name.trim().is_empty() {
        return Err(helpers::ApiError::bad_request(
            "query parameter 'name' must not be empty",
        ));
    }

    Ok(())
}
