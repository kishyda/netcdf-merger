use std::io;

use rocket::http::Status;

#[test]
fn api_error_constructors_preserve_status_and_message() {
    let bad_request = wind_merge::helpers::ApiError::bad_request("bad");
    let not_found = wind_merge::helpers::ApiError::not_found("missing");
    let conflict = wind_merge::helpers::ApiError::conflict("conflict");
    let explicit = wind_merge::helpers::ApiError::new(Status::ImATeapot, "teapot");

    assert_eq!(bad_request.status, Status::BadRequest);
    assert_eq!(bad_request.message, "bad");
    assert_eq!(not_found.status, Status::NotFound);
    assert_eq!(not_found.message, "missing");
    assert_eq!(conflict.status, Status::Conflict);
    assert_eq!(conflict.message, "conflict");
    assert_eq!(explicit.status, Status::ImATeapot);
    assert_eq!(explicit.message, "teapot");
}

#[test]
fn api_error_internal_and_from_error_convert_to_internal_server_error() {
    let internal = wind_merge::helpers::ApiError::internal("boom");
    let converted: wind_merge::helpers::ApiError = io::Error::other("io failure").into();

    assert_eq!(internal.status, Status::InternalServerError);
    assert_eq!(internal.message, "boom");
    assert_eq!(converted.status, Status::InternalServerError);
    assert_eq!(converted.message, "io failure");
}
