from __future__ import annotations

import pathlib
import random
import sys
import tempfile
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
    run_stress_test,
    save_case_artifacts,
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

        summary = run_stress_test(InMemoryMergeClient(), iterations=5, seed=77)

        self.assertEqual(summary, StressSummary(seed=77, total_cases=5, passed_cases=5))

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
