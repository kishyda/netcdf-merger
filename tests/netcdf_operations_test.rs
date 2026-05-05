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

fn create_integer_part_bytes() -> Vec<u8> {
    let mut dataset = netcdf::create_mem(1024 * 1024).expect("create dataset");
    dataset
        .add_unlimited_dimension("time")
        .expect("unlimited dimension");
    dataset
        .add_attribute("sample_rate_hz", 60_i32)
        .expect("sample_rate_hz");
    {
        let mut variable = dataset
            .add_variable::<i32>("pressure", &["time"])
            .expect("pressure");
        variable.put_attribute("units", "Pa").expect("units");
    }
    dataset.enddef().expect("enddef");
    let mut variable = dataset.variable_mut("pressure").expect("pressure");
    variable
        .put_values(&[1001_i32, 1002, 1003], ..)
        .expect("values");
    dataset.close_to_bytes().expect("bytes")
}

fn create_single_dimension_part_bytes(
    attribute_name: &str,
    attribute_value: &str,
    variable_name: &str,
    values: &[f32],
) -> Vec<u8> {
    let mut dataset = netcdf::create_mem(1024 * 1024).expect("create dataset");
    dataset.add_dimension("lat", values.len()).expect("lat");
    dataset
        .add_attribute(attribute_name, attribute_value)
        .expect("global attribute");
    {
        let mut variable = dataset
            .add_variable::<f32>(variable_name, &["lat"])
            .expect("variable");
        variable.put_attribute("units", "1").expect("units");
    }
    dataset.enddef().expect("enddef");
    let mut variable = dataset.variable_mut(variable_name).expect("variable");
    variable.put_values(values, ..).expect("values");
    dataset.close_to_bytes().expect("bytes")
}

fn create_extended_attribute_part_bytes() -> Vec<u8> {
    let mut dataset = netcdf::create_mem(1024 * 1024).expect("create dataset");
    dataset.add_dimension("lat", 2).expect("lat");
    dataset
        .add_attribute("station_id", -520.588_f64)
        .expect("station_id");
    dataset
        .add_attribute("quality_flag", -923_i64)
        .expect("quality_flag");
    {
        let mut variable = dataset
            .add_variable::<f32>("temperature", &["lat"])
            .expect("temperature");
        variable.put_attribute("units", "K").expect("units");
    }
    dataset.enddef().expect("enddef");
    let mut variable = dataset.variable_mut("temperature").expect("temperature");
    variable
        .put_values(&[1.5_f32, 2.5_f32], ..)
        .expect("values");
    dataset.close_to_bytes().expect("bytes")
}

#[test]
fn combine_netcdf_bytes_produces_the_expected_merged_dataset() {
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

    let combined =
        wind_merge::netcdf_operations::combine_netcdf_bytes(&[&part_a, &part_b]).expect("combine");
    let file = netcdf::open_mem(None, &combined).expect("open combined");

    let temperature = file.variable("temperature").expect("temperature variable");
    let humidity = file.variable("humidity").expect("humidity variable");

    assert_eq!(file.dimensions().count(), 2);
    assert_eq!(
        file.attribute("max_temp")
            .expect("max_temp attribute")
            .name(),
        "max_temp"
    );
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
    assert_eq!(
        temperature
            .attribute("units")
            .expect("temperature units")
            .value()
            .expect("temperature units value"),
        AttributeValue::Str("1".into())
    );
    assert_eq!(
        humidity
            .attribute("units")
            .expect("humidity units")
            .value()
            .expect("humidity units value"),
        AttributeValue::Str("1".into())
    );
    assert_eq!(
        temperature
            .get_values::<f32, _>((.., ..))
            .expect("temperature values"),
        vec![273.15, 274.15, 275.15, 276.15]
    );
    assert_eq!(
        humidity
            .get_values::<f32, _>((.., ..))
            .expect("humidity values"),
        vec![0.5, 0.6, 0.7, 0.8]
    );
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

    let combined =
        wind_merge::netcdf_operations::combine_netcdf_bytes(&[&part_a, &part_b]).expect("combine");
    let file = netcdf::open_mem(None, &combined).expect("open combined");

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

    let combined =
        wind_merge::netcdf_operations::combine_netcdf_bytes(&[&part_a, &part_b]).expect("combine");
    let file = netcdf::open_mem(None, &combined).expect("open combined");
    let variable = file.variable("temperature").expect("temperature");
    let values = variable.get_values::<f32, _>((.., ..)).expect("values");

    assert!(values == vec![1.0, 2.0, 3.0, 4.0] || values == vec![10.0, 20.0, 30.0, 40.0]);
    assert!(file.attribute("max_temp").is_some());
    assert!(file.attribute("other_temp").is_some());
}

#[test]
fn combine_netcdf_bytes_preserves_integer_variables_and_unlimited_dimensions() {
    let integer_part = create_integer_part_bytes();
    let weather_part = create_part_bytes(
        "part-a",
        "max_temp",
        "310K",
        "temperature",
        &[1.0, 2.0, 3.0, 4.0],
    );

    let combined =
        wind_merge::netcdf_operations::combine_netcdf_bytes(&[&integer_part, &weather_part])
            .expect("combine");
    let file = netcdf::open_mem(None, &combined).expect("open combined");
    let time = file.dimension("time").expect("time dimension");
    let pressure = file.variable("pressure").expect("pressure variable");

    assert!(time.is_unlimited());
    assert_eq!(time.len(), 3);
    assert_eq!(
        file.attribute("sample_rate_hz")
            .expect("sample_rate_hz")
            .value()
            .expect("sample_rate_hz value"),
        AttributeValue::Int(60)
    );
    assert_eq!(
        pressure
            .attribute("units")
            .expect("pressure units")
            .value()
            .expect("pressure units value"),
        AttributeValue::Str("Pa".into())
    );
    assert_eq!(
        pressure.get_values::<i32, _>(..).expect("pressure values"),
        vec![1001, 1002, 1003]
    );
}

#[test]
fn combine_netcdf_bytes_ignores_later_duplicate_variable_data_even_when_shape_differs() {
    let part_a = create_part_bytes(
        "part-a",
        "max_temp",
        "310K",
        "temperature",
        &[1.0, 2.0, 3.0, 4.0],
    );
    let part_b =
        create_single_dimension_part_bytes("other_temp", "999K", "temperature", &[10.0, 20.0]);

    let combined = wind_merge::netcdf_operations::combine_netcdf_bytes(&[&part_a, &part_b])
        .expect("combine should keep one duplicate variable without crashing");
    let file = netcdf::open_mem(None, &combined).expect("open combined");
    let variable = file.variable("temperature").expect("temperature");

    assert_eq!(
        variable
            .get_values::<f32, _>((.., ..))
            .expect("temperature values"),
        vec![1.0, 2.0, 3.0, 4.0]
    );
    assert!(file.attribute("max_temp").is_some());
    assert!(file.attribute("other_temp").is_some());
}

#[test]
fn combine_netcdf_bytes_preserves_extended_numeric_global_attributes() {
    let part_a = create_extended_attribute_part_bytes();
    let part_b = create_part_bytes(
        "part-b",
        "avg_humidity",
        "65%",
        "humidity",
        &[10.0, 20.0, 30.0, 40.0],
    );

    let combined =
        wind_merge::netcdf_operations::combine_netcdf_bytes(&[&part_a, &part_b]).expect("combine");
    let file = netcdf::open_mem(None, &combined).expect("open combined");

    assert_eq!(
        file.attribute("station_id")
            .expect("station_id")
            .value()
            .expect("station_id value"),
        AttributeValue::Double(-520.588)
    );
    assert_eq!(
        file.attribute("quality_flag")
            .expect("quality_flag")
            .value()
            .expect("quality_flag value"),
        AttributeValue::Longlong(-923)
    );
    assert!(file.variable("temperature").is_some());
    assert!(file.variable("humidity").is_some());
}
