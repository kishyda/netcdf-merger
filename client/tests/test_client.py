from __future__ import annotations

import pathlib
import sys
import unittest

from netCDF4 import Dataset

CLIENT_SRC = pathlib.Path(__file__).resolve().parents[1] / "src"
if str(CLIENT_SRC) not in sys.path:
    sys.path.insert(0, str(CLIENT_SRC))

from windborne_client import (  # noqa: E402
    ApiResponse,
    create_expected_output_netcdf_bytes,
    create_part_a_netcdf_bytes,
    create_part_b_netcdf_bytes,
    describe_dataset,
    dump_dataset,
)


class ClientNetcdfTests(unittest.TestCase):
    def assert_nested_values_almost_equal(
        self,
        actual: list[list[float]],
        expected: list[list[float]],
    ) -> None:
        self.assertEqual(len(actual), len(expected))
        for actual_row, expected_row in zip(actual, expected, strict=True):
            self.assertEqual(len(actual_row), len(expected_row))
            for actual_value, expected_value in zip(actual_row, expected_row, strict=True):
                self.assertAlmostEqual(actual_value, expected_value, delta=1e-4)

    def test_create_part_a_netcdf_bytes_contains_expected_variable_and_attribute(self) -> None:
        dataset = Dataset("inmemory-part-a.nc", mode="r", memory=create_part_a_netcdf_bytes("part-a"))

        try:
            self.assertIn("temperature", dataset.variables)
            self.assertEqual(dataset.max_temp, "310K")
            self.assertEqual(dataset.variables["temperature"].units, "K")
            self.assertEqual(dataset.variables["temperature"].shape, (2, 2))
            self.assert_nested_values_almost_equal(
                dataset.variables["temperature"][:].tolist(),
                [[273.15, 274.15], [275.15, 276.15]],
            )
        finally:
            dataset.close()

    def test_create_part_b_netcdf_bytes_contains_expected_variable_and_attribute(self) -> None:
        dataset = Dataset("inmemory-part-b.nc", mode="r", memory=create_part_b_netcdf_bytes("part-b"))

        try:
            self.assertIn("humidity", dataset.variables)
            self.assertEqual(dataset.avg_humidity, "65%")
            self.assertEqual(dataset.variables["humidity"].units, "%")
            self.assertEqual(dataset.variables["humidity"].shape, (2, 2))
            self.assert_nested_values_almost_equal(
                dataset.variables["humidity"][:].tolist(),
                [[0.45, 0.55], [0.65, 0.75]],
            )
        finally:
            dataset.close()

    def test_create_expected_output_netcdf_bytes_contains_merged_variables_and_attributes(self) -> None:
        dataset = Dataset(
            "inmemory-expected-output.nc",
            mode="r",
            memory=create_expected_output_netcdf_bytes("merged-dataset"),
        )

        try:
            self.assertEqual(dataset.title, "merged-dataset")
            self.assertEqual(dataset.max_temp, "310K")
            self.assertEqual(dataset.avg_humidity, "65%")
            self.assertIn("temperature", dataset.variables)
            self.assertIn("humidity", dataset.variables)
            self.assertEqual(dataset.variables["temperature"].units, "K")
            self.assertEqual(dataset.variables["humidity"].units, "%")
            self.assert_nested_values_almost_equal(
                dataset.variables["temperature"][:].tolist(),
                [[273.15, 274.15], [275.15, 276.15]],
            )
            self.assert_nested_values_almost_equal(
                dataset.variables["humidity"][:].tolist(),
                [[0.45, 0.55], [0.65, 0.75]],
            )
        finally:
            dataset.close()

    def test_describe_dataset_lists_variables_and_global_attributes(self) -> None:
        dataset = Dataset(
            "inmemory-describe.nc",
            mode="r",
            memory=create_expected_output_netcdf_bytes("described-dataset"),
        )

        try:
            description = describe_dataset(dataset)
        finally:
            dataset.close()

        self.assertIn("title=described-dataset", description)
        self.assertIn("temperature", description)
        self.assertIn("humidity", description)
        self.assertIn("attrs=[title, max_temp, avg_humidity]", description)

    def test_dump_dataset_includes_values_and_metadata(self) -> None:
        dataset = Dataset(
            "inmemory-dump.nc",
            mode="r",
            memory=create_expected_output_netcdf_bytes("dumped-dataset"),
        )

        try:
            dump = dump_dataset(dataset)
        finally:
            dataset.close()

        self.assertIn("Global attributes:", dump)
        self.assertIn("Variables:", dump)
        self.assertIn("temperature", dump)
        self.assertIn("humidity", dump)
        self.assertIn("values =", dump)

    def test_api_response_helpers_behave_as_expected(self) -> None:
        response = ApiResponse(
            status=200,
            content_type="application/netcdf",
            body=b"hello",
        )

        self.assertEqual(response.text, "hello")
        self.assertTrue(response.is_netcdf())


if __name__ == "__main__":
    unittest.main()
