from __future__ import annotations

from netCDF4 import Dataset


def create_part_a_netcdf_bytes(dataset_name: str) -> bytes:
    dataset = Dataset("client.nc", mode="w", diskless=True, memory=1024 * 1024)
    dataset.createDimension("lat", 2)
    dataset.createDimension("lon", 2)

    temperatures = dataset.createVariable("temperature", "f4", ("lat", "lon"))
    temperatures.units = "K"
    dataset.title = dataset_name
    dataset.max_temp = "310K"
    temperatures[:, :] = [[273.15, 274.15], [275.15, 276.15]]

    memory_view = dataset.close()
    return bytes(memory_view)


def create_part_b_netcdf_bytes(dataset_name: str) -> bytes:
    dataset = Dataset("client.nc", mode="w", diskless=True, memory=1024 * 1024)
    dataset.createDimension("lat", 2)
    dataset.createDimension("lon", 2)

    humidity = dataset.createVariable("humidity", "f4", ("lat", "lon"))
    humidity.units = "%"
    dataset.title = dataset_name
    dataset.avg_humidity = "65%"
    humidity[:, :] = [[0.45, 0.55], [0.65, 0.75]]

    memory_view = dataset.close()
    return bytes(memory_view)


def create_expected_output_netcdf_bytes(dataset_name: str) -> bytes:
    dataset = Dataset("client.nc", mode="w", diskless=True, memory=1024 * 1024)
    dataset.createDimension("lat", 2)
    dataset.createDimension("lon", 2)

    temperatures = dataset.createVariable("temperature", "f4", ("lat", "lon"))
    temperatures.units = "K"
    temperatures[:, :] = [[273.15, 274.15], [275.15, 276.15]]

    humidity = dataset.createVariable("humidity", "f4", ("lat", "lon"))
    humidity.units = "%"
    humidity[:, :] = [[0.45, 0.55], [0.65, 0.75]]

    dataset.title = dataset_name
    dataset.max_temp = "310K"
    dataset.avg_humidity = "65%"

    memory_view = dataset.close()
    return bytes(memory_view)
