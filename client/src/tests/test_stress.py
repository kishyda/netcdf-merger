from __future__ import annotations

import pathlib
import random
import sys
import tempfile
import threading
import unittest

from netCDF4 import Dataset

CLIENT_SRC = pathlib.Path(__file__).resolve().parents[1]
if str(CLIENT_SRC) not in sys.path:
    sys.path.insert(0, str(CLIENT_SRC))

from windborne_client import ApiResponse, WindborneClient  # noqa: E402
from windborne_client.stress import (  # noqa: E402
    DatasetSpec,
    StressSummary,
    VariableSpec,
    create_netcdf_bytes_from_spec,
    create_random_stress_case,
    create_requirement_compliant_merge_bytes,
    format_stress_profile,
    run_stress_test,
    save_case_artifacts,
    stress_profile_recommendations,
    verify_merged_dataset,
)


class StressTests(unittest.TestCase):
    def test_create_random_stress_case_produces_readable_netcdf_files(self) -> None:
        case = create_random_stress_case(random.Random(1234), "random-case")

        part_a = Dataset("part-a.nc", mode="r", memory=case.part_a_bytes)
        part_b = Dataset("part-b.nc", mode="r", memory=case.part_b_bytes)

        try:
            self.assertGreaterEqual(len(part_a.variables), 1)
            self.assertGreaterEqual(len(part_b.variables), 1)
        finally:
            part_a.close()
            part_b.close()

    def test_verify_merged_dataset_accepts_prefer_part_a_merge(self) -> None:
        part_a_spec = DatasetSpec(
            dimensions={"lat": 2, "lon": 2},
            global_attributes={"title": "part-a", "shared": "a"},
            variables=(
                VariableSpec(
                    name="temperature",
                    dtype="f4",
                    dimensions=("lat", "lon"),
                    attributes={"units": "K"},
                    values=[[1.0, 2.0], [3.0, 4.0]],
                ),
                VariableSpec(
                    name="shared_var",
                    dtype="i4",
                    dimensions=("lat",),
                    attributes={"units": "count"},
                    values=[1, 2],
                ),
            ),
        )
        part_b_spec = DatasetSpec(
            dimensions={"lat": 2, "lon": 2},
            global_attributes={"title": "part-b", "extra": "b"},
            variables=(
                VariableSpec(
                    name="humidity",
                    dtype="f4",
                    dimensions=("lat", "lon"),
                    attributes={"units": "%"},
                    values=[[0.1, 0.2], [0.3, 0.4]],
                ),
                VariableSpec(
                    name="shared_var",
                    dtype="i4",
                    dimensions=("lat",),
                    attributes={"units": "count"},
                    values=[9, 8],
                ),
            ),
        )

        merged = Dataset(
            "merged-a.nc",
            mode="r",
            memory=create_requirement_compliant_merge_bytes(
                part_a_spec,
                part_b_spec,
                duplicate_policy="prefer_part_a",
            ),
        )

        try:
            verify_merged_dataset(merged, part_a_spec, part_b_spec)
        finally:
            merged.close()

    def test_verify_merged_dataset_accepts_prefer_part_b_merge(self) -> None:
        part_a_spec = DatasetSpec(
            dimensions={"lat": 2, "lon": 2},
            global_attributes={"title": "part-a", "shared": "a"},
            variables=(
                VariableSpec(
                    name="temperature",
                    dtype="f4",
                    dimensions=("lat", "lon"),
                    attributes={"units": "K"},
                    values=[[1.0, 2.0], [3.0, 4.0]],
                ),
            ),
        )
        part_b_spec = DatasetSpec(
            dimensions={"lat": 2, "lon": 2},
            global_attributes={"title": "part-b", "shared": "b"},
            variables=(
                VariableSpec(
                    name="temperature",
                    dtype="f4",
                    dimensions=("lat", "lon"),
                    attributes={"units": "degK"},
                    values=[[5.0, 6.0], [7.0, 8.0]],
                ),
                VariableSpec(
                    name="humidity",
                    dtype="f4",
                    dimensions=("lat", "lon"),
                    attributes={"units": "%"},
                    values=[[0.1, 0.2], [0.3, 0.4]],
                ),
            ),
        )

        merged = Dataset(
            "merged-b.nc",
            mode="r",
            memory=create_requirement_compliant_merge_bytes(
                part_a_spec,
                part_b_spec,
                duplicate_policy="prefer_part_b",
            ),
        )

        try:
            verify_merged_dataset(merged, part_a_spec, part_b_spec)
        finally:
            merged.close()

    def test_run_stress_test_uploads_cases_and_verifies_results(self) -> None:
        class InMemoryMergeClient(WindborneClient):
            def __init__(self) -> None:
                super().__init__("http://example.com")
                self.uploaded: dict[str, bytes] = {}

            def upload(  # type: ignore[override]
                self,
                endpoint: str,
                name: str,
                dataset_name: str,
                netcdf_bytes: bytes | None = None,
            ) -> ApiResponse:
                self.uploaded[f"{name}:{endpoint}"] = netcdf_bytes or b""
                return ApiResponse(status=200, content_type="text/plain", body=b"ok")

            def read_response(self, name: str) -> ApiResponse:  # type: ignore[override]
                part_a = self.uploaded[f"{name}:part_a"]
                part_b = self.uploaded[f"{name}:part_b"]
                return ApiResponse(
                    status=200,
                    content_type="application/netcdf",
                    body=create_requirement_compliant_merge_bytes(
                        _spec_from_bytes(part_a),
                        _spec_from_bytes(part_b),
                        duplicate_policy="prefer_part_b",
                    ),
                )

        summary = run_stress_test(
            InMemoryMergeClient(),
            iterations=5,
            seed=77,
            parallelism=1,
            progress=False,
        )

        self.assertEqual(summary.seed, 77)
        self.assertEqual(summary.total_cases, 5)
        self.assertEqual(summary.passed_cases, 5)
        self.assertEqual(summary.failed_cases, 0)
        self.assertEqual(summary.parallelism, 1)
        self.assertEqual(summary.total_requests, 15)
        self.assertGreater(summary.elapsed_seconds, 0)
        self.assertGreater(summary.cases_per_second, 0)
        self.assertGreater(summary.requests_per_second, 0)
        self.assertGreater(summary.average_case_seconds, 0)
        self.assertGreater(summary.peak_python_memory_bytes, 0)

    def test_run_stress_test_uploads_parts_in_parallel(self) -> None:
        class BlockingUploadClient(WindborneClient):
            def __init__(self) -> None:
                super().__init__("http://example.com")
                self.lock = threading.Lock()
                self.uploaded: dict[str, bytes] = {}
                self.upload_count = 0
                self.both_uploads_started = threading.Event()

            def upload(  # type: ignore[override]
                self,
                endpoint: str,
                name: str,
                dataset_name: str,
                netcdf_bytes: bytes | None = None,
            ) -> ApiResponse:
                with self.lock:
                    self.upload_count += 1
                    self.uploaded[f"{name}:{endpoint}"] = netcdf_bytes or b""
                    if self.upload_count == 2:
                        self.both_uploads_started.set()

                if not self.both_uploads_started.wait(timeout=1):
                    raise AssertionError("part uploads were not started in parallel")

                return ApiResponse(status=200, content_type="text/plain", body=b"ok")

            def read_response(self, name: str) -> ApiResponse:  # type: ignore[override]
                part_a = self.uploaded[f"{name}:part_a"]
                part_b = self.uploaded[f"{name}:part_b"]
                return ApiResponse(
                    status=200,
                    content_type="application/netcdf",
                    body=create_requirement_compliant_merge_bytes(
                        _spec_from_bytes(part_a),
                        _spec_from_bytes(part_b),
                        duplicate_policy="prefer_part_b",
                    ),
                )

        summary = run_stress_test(
            BlockingUploadClient(),
            iterations=1,
            seed=77,
            parallelism=1,
            progress=False,
        )

        self.assertEqual(summary.seed, 77)
        self.assertEqual(summary.total_cases, 1)
        self.assertEqual(summary.passed_cases, 1)
        self.assertEqual(summary.parallelism, 1)

    def test_stress_profile_formats_extrapolation_numbers(self) -> None:
        summary = StressSummary(
            seed=9,
            total_cases=10,
            passed_cases=10,
            parallelism=4,
            elapsed_seconds=2.0,
            peak_python_memory_bytes=1024 * 1024,
            peak_rss_bytes=2 * 1024 * 1024,
        )

        self.assertEqual(summary.total_requests, 30)
        self.assertEqual(summary.cases_per_second, 5.0)
        self.assertEqual(summary.requests_per_second, 15.0)
        self.assertEqual(summary.average_case_seconds, 0.2)
        self.assertEqual(summary.estimated_seconds_for_cases(25), 5.0)
        self.assertIn("throughput=5.00 cases/s", format_stress_profile(summary))
        self.assertIn("peak_python_memory=1.0 MiB", format_stress_profile(summary))
        self.assertGreaterEqual(len(stress_profile_recommendations()), 1)

    def test_save_case_artifacts_writes_manifest_and_netcdf_files(self) -> None:
        case = create_random_stress_case(random.Random(123), "artifact-case")

        with tempfile.TemporaryDirectory() as tmpdir:
            save_case_artifacts(
                pathlib.Path(tmpdir),
                case=case,
                index=3,
                seed=99,
                merged_bytes=case.part_a_bytes,
                status="failed",
                error_message="boom",
            )

            case_dir = pathlib.Path(tmpdir) / "0003-artifact-case"
            self.assertTrue((case_dir / "part_a.nc").exists())
            self.assertTrue((case_dir / "part_b.nc").exists())
            self.assertTrue((case_dir / "merged.nc").exists())
            self.assertTrue((case_dir / "manifest.json").exists())
            manifest = (case_dir / "manifest.json").read_text()
            self.assertIn('"status": "failed"', manifest)
            self.assertIn('"merged_spec": {', manifest)
            self.assertIn('"variables": [', manifest)


def _spec_from_bytes(netcdf_bytes: bytes) -> DatasetSpec:
    dataset = Dataset("spec.nc", mode="r", memory=netcdf_bytes)

    try:
        return DatasetSpec(
            dimensions={name: len(dimension) for name, dimension in dataset.dimensions.items()},
            global_attributes={
                attribute_name: getattr(dataset, attribute_name)
                for attribute_name in dataset.ncattrs()
            },
            variables=tuple(
                VariableSpec(
                    name=variable_name,
                    dtype=_dtype_to_spec_name(str(variable.dtype)),
                    dimensions=tuple(variable.dimensions),
                    attributes={
                        attribute_name: getattr(variable, attribute_name)
                        for attribute_name in variable.ncattrs()
                    },
                    values=variable[:].tolist(),
                )
                for variable_name, variable in dataset.variables.items()
            ),
        )
    finally:
        dataset.close()


def _dtype_to_spec_name(dtype: str) -> str:
    if dtype == "float32":
        return "f4"
    if dtype == "int32":
        return "i4"
    return dtype


if __name__ == "__main__":
    unittest.main()
