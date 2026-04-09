from __future__ import annotations

from netCDF4 import Dataset

from .api import WindborneClient
from .display import describe_dataset, dump_dataset
from .samples import create_expected_output_netcdf_bytes


def main() -> None:
    client = WindborneClient()
    combined_name = "combined-example"

    part_a_response = client.part_a(
        name=combined_name,
        dataset_name="Created by client for /part_a",
    )
    print(f"POST /part_a -> {part_a_response.text}")

    part_b_response = client.part_b(
        name=combined_name,
        dataset_name="Created by client for /part_b",
    )
    print(f"POST /part_b -> {part_b_response.text}")

    combined_dataset = client.read(name=combined_name)
    expected_dataset = Dataset(
        "inmemory.nc",
        mode="r",
        diskless=True,
        memory=create_expected_output_netcdf_bytes(""),
    )

    print(f"Combined Dataset: {dump_dataset(combined_dataset)}\n\n")
    print(f"Expected Dataset: {dump_dataset(expected_dataset)}\n\n")
    print(f"GET /read merged -> {describe_dataset(combined_dataset)}")


if __name__ == "__main__":
    main()
