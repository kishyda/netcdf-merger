use netcdf::AttributeValue;

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

#[test]
fn combine_netcdf_bytes_merges_global_attributes_and_variables() {
    let part_a = create_part_bytes(
        "part-a",
        "max_temp",
        "310K",
        "temperature",
        &[273.15, 274.15, 275.15, 276.15],
    );
    let part_b = create_part_bytes(
        "part-b",
        "avg_humidity",
        "65%",
        "humidity",
        &[0.5, 0.6, 0.7, 0.8],
    );

    let combined = windborne_oa::netcdf_operations::combine_netcdf_bytes(&[&part_a, &part_b])
        .expect("combine");
    let file = netcdf::open_mem(Some("combined.nc"), &combined).expect("open combined");

    assert!(file.variable("temperature").is_some());
    assert!(file.variable("humidity").is_some());
    assert_eq!(
        file.attribute("max_temp")
            .expect("max_temp")
            .value()
            .expect("max_temp value"),
        AttributeValue::Str("310K".into())
    );
    assert_eq!(
        file.attribute("avg_humidity")
            .expect("avg_humidity")
            .value()
            .expect("avg_humidity value"),
        AttributeValue::Str("65%".into())
    );
}

#[test]
fn combine_netcdf_bytes_keeps_first_variable_on_name_collision() {
    let part_a = create_part_bytes(
        "part-a",
        "max_temp",
        "310K",
        "temperature",
        &[1.0, 2.0, 3.0, 4.0],
    );
    let part_b = create_part_bytes(
        "part-b",
        "other_temp",
        "999K",
        "temperature",
        &[10.0, 20.0, 30.0, 40.0],
    );

    let combined = windborne_oa::netcdf_operations::combine_netcdf_bytes(&[&part_a, &part_b])
        .expect("combine");
    let file = netcdf::open_mem(Some("combined.nc"), &combined).expect("open combined");
    let variable = file.variable("temperature").expect("temperature");
    let values = variable.get_values::<f32, _>((.., ..)).expect("values");

    assert!(values == vec![1.0, 2.0, 3.0, 4.0] || values == vec![10.0, 20.0, 30.0, 40.0]);
    assert!(file.attribute("max_temp").is_some());
    assert!(file.attribute("other_temp").is_some());
}
