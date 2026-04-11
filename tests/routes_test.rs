use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;

fn create_part_bytes(
    title: &str,
    attribute_name: &str,
    attribute_value: &str,
    variable_name: &str,
    values: &[f32],
) -> Vec<u8> {
    let mut dataset = netcdf::create_mem(1024 * 1024).expect("create dataset");
    dataset.add_dimension("lat", 2).expect("lat");
    dataset.add_dimension("lon", 2).expect("lon");
    dataset.add_attribute("title", title).expect("title");
    dataset
        .add_attribute(attribute_name, attribute_value)
        .expect("global attribute");
    {
        let mut variable = dataset
            .add_variable::<f32>(variable_name, &["lat", "lon"])
            .expect("variable");
        variable.put_attribute("units", "1").expect("units");
    }
    dataset.enddef().expect("enddef");
    let mut variable = dataset.variable_mut(variable_name).expect("variable");
    variable.put_values(values, (.., ..)).expect("values");
    dataset.close_to_bytes().expect("bytes")
}

async fn build_client() -> Client {
    Client::tracked(windborne_oa::build_rocket())
        .await
        .expect("valid rocket")
}

#[rocket::async_test]
async fn part_a_rejects_blank_name() {
    let client = build_client().await;

    let response = client
        .post("/part_a?name=%20%20%20")
        .header(ContentType::new("application", "netcdf"))
        .body(create_part_bytes(
            "part-a",
            "max_temp",
            "310K",
            "temperature",
            &[1.0, 2.0, 3.0, 4.0],
        ))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn part_a_rejects_invalid_netcdf_body() {
    let client = build_client().await;

    let response = client
        .post("/part_a?name=invalid")
        .header(ContentType::new("application", "netcdf"))
        .body(b"not a netcdf file".to_vec())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn part_b_rejects_invalid_netcdf_body() {
    let client = build_client().await;

    let response = client
        .post("/part_b?name=invalid")
        .header(ContentType::new("application", "netcdf"))
        .body(b"not a netcdf file".to_vec())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn read_returns_not_found_when_no_parts_exist() {
    let client = build_client().await;

    let response = client.get("/read?name=missing").dispatch().await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn read_rejects_blank_name() {
    let client = build_client().await;

    let response = client.get("/read?name=%20%20%20").dispatch().await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn read_returns_conflict_when_only_one_part_exists() {
    let client = build_client().await;

    let upload = client
        .post("/part_a?name=only-a")
        .header(ContentType::new("application", "netcdf"))
        .body(create_part_bytes(
            "part-a",
            "max_temp",
            "310K",
            "temperature",
            &[1.0, 2.0, 3.0, 4.0],
        ))
        .dispatch()
        .await;
    assert_eq!(upload.status(), Status::Ok);

    let response = client.get("/read?name=only-a").dispatch().await;

    assert_eq!(response.status(), Status::Conflict);
}

#[rocket::async_test]
async fn full_upload_flow_returns_merged_netcdf() {
    let client = build_client().await;

    let part_a = client
        .post("/part_a?name=merged")
        .header(ContentType::new("application", "netcdf"))
        .body(create_part_bytes(
            "part-a",
            "max_temp",
            "310K",
            "temperature",
            &[1.0, 2.0, 3.0, 4.0],
        ))
        .dispatch()
        .await;
    assert_eq!(part_a.status(), Status::Ok);

    let part_b = client
        .post("/part_b?name=merged")
        .header(ContentType::new("application", "netcdf"))
        .body(create_part_bytes(
            "part-b",
            "avg_humidity",
            "65%",
            "humidity",
            &[10.0, 20.0, 30.0, 40.0],
        ))
        .dispatch()
        .await;
    assert_eq!(part_b.status(), Status::Ok);

    let response = client.get("/read?name=merged").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.content_type(),
        Some(ContentType::new("application", "netcdf"))
    );

    let bytes = response.into_bytes().await.expect("response bytes");
    let file = netcdf::open_mem(Some("merged.nc"), &bytes).expect("merged file");

    assert!(file.variable("temperature").is_some());
    assert!(file.variable("humidity").is_some());
    assert!(file.attribute("max_temp").is_some());
    assert!(file.attribute("avg_humidity").is_some());
}

#[rocket::async_test]
async fn read_recomputes_after_upload_invalidates_cached_merge() {
    let client = build_client().await;

    let first_part_a = client
        .post("/part_a?name=cached-merge")
        .header(ContentType::new("application", "netcdf"))
        .body(create_part_bytes(
            "part-a",
            "max_temp",
            "310K",
            "temperature",
            &[1.0, 2.0, 3.0, 4.0],
        ))
        .dispatch()
        .await;
    assert_eq!(first_part_a.status(), Status::Ok);

    let first_part_b = client
        .post("/part_b?name=cached-merge")
        .header(ContentType::new("application", "netcdf"))
        .body(create_part_bytes(
            "part-b",
            "avg_humidity",
            "65%",
            "humidity",
            &[10.0, 20.0, 30.0, 40.0],
        ))
        .dispatch()
        .await;
    assert_eq!(first_part_b.status(), Status::Ok);

    let first_read = client.get("/read?name=cached-merge").dispatch().await;
    assert_eq!(first_read.status(), Status::Ok);

    let updated_part_b = client
        .post("/part_b?name=cached-merge")
        .header(ContentType::new("application", "netcdf"))
        .body(create_part_bytes(
            "part-b-updated",
            "avg_humidity",
            "72%",
            "humidity",
            &[100.0, 200.0, 300.0, 400.0],
        ))
        .dispatch()
        .await;
    assert_eq!(updated_part_b.status(), Status::Ok);

    let second_read = client.get("/read?name=cached-merge").dispatch().await;
    assert_eq!(second_read.status(), Status::Ok);

    let bytes = second_read.into_bytes().await.expect("response bytes");
    let file = netcdf::open_mem(Some("merged.nc"), &bytes).expect("merged file");
    let humidity = file.variable("humidity").expect("humidity variable");

    assert_eq!(
        humidity
            .get_values::<f32, _>((.., ..))
            .expect("humidity values"),
        vec![100.0, 200.0, 300.0, 400.0]
    );
    assert_eq!(
        file.attribute("avg_humidity")
            .expect("avg_humidity")
            .value()
            .expect("avg_humidity value"),
        netcdf::AttributeValue::Str("72%".into())
    );
}

#[rocket::async_test]
async fn repeated_read_uses_cached_merged_dataset() {
    let client = build_client().await;

    let part_a = client
        .post("/part_a?name=cache-hit")
        .header(ContentType::new("application", "netcdf"))
        .body(create_part_bytes(
            "part-a",
            "max_temp",
            "310K",
            "temperature",
            &[1.0, 2.0, 3.0, 4.0],
        ))
        .dispatch()
        .await;
    assert_eq!(part_a.status(), Status::Ok);

    let part_b = client
        .post("/part_b?name=cache-hit")
        .header(ContentType::new("application", "netcdf"))
        .body(create_part_bytes(
            "part-b",
            "avg_humidity",
            "65%",
            "humidity",
            &[10.0, 20.0, 30.0, 40.0],
        ))
        .dispatch()
        .await;
    assert_eq!(part_b.status(), Status::Ok);

    let first_bytes = client
        .get("/read?name=cache-hit")
        .dispatch()
        .await
        .into_bytes()
        .await
        .expect("first read bytes");
    let second_bytes = client
        .get("/read?name=cache-hit")
        .dispatch()
        .await
        .into_bytes()
        .await
        .expect("second read bytes");

    assert_eq!(first_bytes, second_bytes);
}
