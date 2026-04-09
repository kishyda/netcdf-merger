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
async fn read_returns_not_found_when_no_parts_exist() {
    let client = build_client().await;

    let response = client.get("/read?name=missing").dispatch().await;

    assert_eq!(response.status(), Status::NotFound);
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
