from __future__ import annotations

import pathlib
import sys
import unittest
from unittest.mock import patch
from urllib.error import HTTPError, URLError

from netCDF4 import Dataset

CLIENT_SRC = pathlib.Path(__file__).resolve().parents[1]
if str(CLIENT_SRC) not in sys.path:
    sys.path.insert(0, str(CLIENT_SRC))

from windborne_client import api as api_module  # noqa: E402
from windborne_client import (  # noqa: E402
    ApiResponse,
    WindborneClient,
    create_netcdf_bytes_for_endpoint,
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
        self.assertFalse(ApiResponse(status=200, content_type="text/plain", body=b"plain").is_netcdf())

    def test_create_netcdf_bytes_for_endpoint_uses_endpoint_specific_builder(self) -> None:
        dataset = Dataset(
            "inmemory-endpoint.nc",
            mode="r",
            memory=create_netcdf_bytes_for_endpoint("part_b", "endpoint-specific"),
        )

        try:
            self.assertIn("humidity", dataset.variables)
            self.assertEqual(dataset.avg_humidity, "65%")
        finally:
            dataset.close()

    def test_create_netcdf_bytes_for_endpoint_rejects_unknown_endpoint(self) -> None:
        with self.assertRaises(ValueError):
            create_netcdf_bytes_for_endpoint("unknown", "dataset")

    def test_upload_targets_the_requested_endpoint(self) -> None:
        captured: dict[str, object] = {}

        class RecordingClient(WindborneClient):
            def _request(  # type: ignore[override]
                self,
                method: str,
                path: str,
                data: bytes | None = None,
                headers: dict[str, str] | None = None,
            ) -> ApiResponse:
                captured["method"] = method
                captured["path"] = path
                captured["data"] = data
                captured["headers"] = headers
                return ApiResponse(status=200, content_type="text/plain", body=b"ok")

        client = RecordingClient("http://example.com")
        response = client.upload("part_b", name="demo", dataset_name="dataset")

        self.assertEqual(response.text, "ok")
        self.assertEqual(captured["method"], "POST")
        self.assertEqual(captured["path"], "/part_b?name=demo")
        self.assertEqual(captured["headers"], {"Content-Type": "application/netcdf"})
        self.assertIsInstance(captured["data"], bytes)

    def test_part_wrappers_target_the_matching_endpoint(self) -> None:
        captured_paths: list[str] = []

        class RecordingClient(WindborneClient):
            def _request(  # type: ignore[override]
                self,
                method: str,
                path: str,
                data: bytes | None = None,
                headers: dict[str, str] | None = None,
            ) -> ApiResponse:
                captured_paths.append(path)
                return ApiResponse(status=200, content_type="text/plain", body=b"ok")

        client = RecordingClient("http://example.com")
        client.part_a(name="dataset-a", dataset_name="part-a")
        client.part_b(name="dataset-b", dataset_name="part-b")

        self.assertEqual(captured_paths, ["/part_a?name=dataset-a", "/part_b?name=dataset-b"])

    def test_read_returns_dataset_when_server_responds_with_netcdf(self) -> None:
        expected_bytes = create_expected_output_netcdf_bytes("server-read")

        class RecordingClient(WindborneClient):
            def _request(  # type: ignore[override]
                self,
                method: str,
                path: str,
                data: bytes | None = None,
                headers: dict[str, str] | None = None,
            ) -> ApiResponse:
                self.last_request = (method, path, data, headers)
                return ApiResponse(
                    status=200,
                    content_type="application/netcdf",
                    body=expected_bytes,
                )

        client = RecordingClient("http://example.com")
        dataset = client.read("server-read")

        try:
            self.assertEqual(client.last_request, ("GET", "/read?name=server-read", None, None))
            self.assertIn("temperature", dataset.variables)
            self.assertIn("humidity", dataset.variables)
        finally:
            dataset.close()

    def test_dataset_from_response_rejects_non_netcdf_content(self) -> None:
        response = ApiResponse(status=200, content_type="text/plain", body=b"nope")

        with self.assertRaisesRegex(RuntimeError, "Expected application/netcdf"):
            WindborneClient._dataset_from_response(response)

    def test_request_wraps_http_errors(self) -> None:
        client = WindborneClient("http://example.com")
        error = HTTPError(
            url="http://example.com/test",
            code=400,
            msg="Bad Request",
            hdrs=None, #type: ignore
            fp=None,
        )
        error.read = lambda: b"bad body"  # type: ignore[method-assign]

        with patch.object(api_module, "urlopen", side_effect=error):
            with self.assertRaisesRegex(RuntimeError, r"HTTP 400: bad body"):
                client._request("GET", "/test")

    def test_request_wraps_url_errors(self) -> None:
        client = WindborneClient("http://example.com")

        with patch.object(
            api_module,
            "urlopen",
            side_effect=URLError("connection refused"),
        ):
            with self.assertRaisesRegex(RuntimeError, "Request failed: connection refused"):
                client._request("GET", "/test")


if __name__ == "__main__":
    unittest.main()
