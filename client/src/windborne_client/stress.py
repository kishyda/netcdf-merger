from __future__ import annotations

import argparse
import dataclasses
import json
import random
from pathlib import Path
from typing import Any

from netCDF4 import Dataset

from .api import WindborneClient


ScalarValue = str | int | float


@dataclasses.dataclass(frozen=True)
class VariableSpec:
    name: str
    dtype: str
    dimensions: tuple[str, ...]
    attributes: dict[str, ScalarValue]
    values: Any


@dataclasses.dataclass(frozen=True)
class DatasetSpec:
    dimensions: dict[str, int]
    global_attributes: dict[str, ScalarValue]
    variables: tuple[VariableSpec, ...]


@dataclasses.dataclass(frozen=True)
class StressCase:
    name: str
    part_a_spec: DatasetSpec
    part_b_spec: DatasetSpec
    part_a_bytes: bytes
    part_b_bytes: bytes


@dataclasses.dataclass(frozen=True)
class StressSummary:
    seed: int
    total_cases: int
    passed_cases: int

    @property
    def failed_cases(self) -> int:
        return self.total_cases - self.passed_cases


def create_random_stress_case(rng: random.Random, name: str) -> StressCase:
    dims = _random_dimensions(rng)
    variable_names = (
        "temperature",
        "humidity",
        "pressure",
        "wind_speed",
        "rainfall",
        "cloud_cover",
    )
    attribute_names = (
        "title",
        "max_temp",
        "avg_humidity",
        "station_id",
        "quality_flag",
        "source",
    )

    part_a_variables = _random_variable_specs(
        rng=rng,
        dimensions=dims,
        variable_names=variable_names,
        count=rng.randint(1, 3),
    )
    part_a_attributes = _random_global_attributes(
        rng=rng,
        attribute_names=attribute_names,
        count=rng.randint(1, 3),
        defaults={"title": f"{name}-part-a"},
    )

    part_b_variables = _random_variable_specs(
        rng=rng,
        dimensions=dims,
        variable_names=variable_names,
        count=rng.randint(1, 3),
    )
    part_b_attributes = _random_global_attributes(
        rng=rng,
        attribute_names=attribute_names,
        count=rng.randint(1, 3),
        defaults={"title": f"{name}-part-b"},
    )

    part_a_spec = DatasetSpec(
        dimensions=dims,
        global_attributes=part_a_attributes,
        variables=tuple(part_a_variables),
    )
    part_b_spec = DatasetSpec(
        dimensions=dims,
        global_attributes=part_b_attributes,
        variables=tuple(part_b_variables),
    )

    return StressCase(
        name=name,
        part_a_spec=part_a_spec,
        part_b_spec=part_b_spec,
        part_a_bytes=create_netcdf_bytes_from_spec(part_a_spec),
        part_b_bytes=create_netcdf_bytes_from_spec(part_b_spec),
    )


def create_netcdf_bytes_from_spec(spec: DatasetSpec) -> bytes:
    dataset = Dataset("stress.nc", mode="w", diskless=True, memory=1024 * 1024)

    for dimension_name, length in spec.dimensions.items():
        dataset.createDimension(dimension_name, length)

    for attribute_name, attribute_value in spec.global_attributes.items():
        setattr(dataset, attribute_name, attribute_value)

    for variable_spec in spec.variables:
        variable = dataset.createVariable(
            variable_spec.name,
            variable_spec.dtype,
            variable_spec.dimensions,
            fill_value=False,
        )
        for attribute_name, attribute_value in variable_spec.attributes.items():
            setattr(variable, attribute_name, attribute_value)
        variable[:] = variable_spec.values

    memory_view = dataset.close()
    return bytes(memory_view)


def create_requirement_compliant_merge_bytes(
    part_a_spec: DatasetSpec,
    part_b_spec: DatasetSpec,
    *,
    duplicate_policy: str = "prefer_part_a",
) -> bytes:
    if duplicate_policy not in {"prefer_part_a", "prefer_part_b"}:
        raise ValueError(f"unsupported duplicate policy: {duplicate_policy!r}")

    merged_dimensions = dict(part_a_spec.dimensions)
    for name, length in part_b_spec.dimensions.items():
        merged_dimensions.setdefault(name, length)

    if duplicate_policy == "prefer_part_a":
        merged_attributes = dict(part_b_spec.global_attributes)
        merged_attributes.update(part_a_spec.global_attributes)
        merged_variables = {variable.name: variable for variable in part_b_spec.variables}
        merged_variables.update({variable.name: variable for variable in part_a_spec.variables})
    else:
        merged_attributes = dict(part_a_spec.global_attributes)
        merged_attributes.update(part_b_spec.global_attributes)
        merged_variables = {variable.name: variable for variable in part_a_spec.variables}
        merged_variables.update({variable.name: variable for variable in part_b_spec.variables})

    spec = DatasetSpec(
        dimensions=merged_dimensions,
        global_attributes=merged_attributes,
        variables=tuple(merged_variables.values()),
    )
    return create_netcdf_bytes_from_spec(spec)


def verify_merged_dataset(
    merged_dataset: Dataset,
    part_a_spec: DatasetSpec,
    part_b_spec: DatasetSpec,
) -> None:
    expected_dimensions = dict(part_a_spec.dimensions)
    for name, length in part_b_spec.dimensions.items():
        expected_dimensions.setdefault(name, length)

    actual_dimension_lengths = {
        name: len(dimension) for name, dimension in merged_dataset.dimensions.items()
    }
    if actual_dimension_lengths != expected_dimensions:
        raise AssertionError(
            f"dimension mismatch: expected {expected_dimensions}, got {actual_dimension_lengths}"
        )

    _assert_attributes_match_requirement(
        actual_attributes={
            attribute_name: _normalize_value(getattr(merged_dataset, attribute_name))
            for attribute_name in merged_dataset.ncattrs()
        },
        part_a_attributes=part_a_spec.global_attributes,
        part_b_attributes=part_b_spec.global_attributes,
        label="global attributes",
    )

    actual_variable_names = set(merged_dataset.variables.keys())
    expected_variable_names = {
        variable.name for variable in part_a_spec.variables
    } | {variable.name for variable in part_b_spec.variables}
    if actual_variable_names != expected_variable_names:
        raise AssertionError(
            f"variable mismatch: expected {sorted(expected_variable_names)}, got {sorted(actual_variable_names)}"
        )

    part_a_variables = {variable.name: variable for variable in part_a_spec.variables}
    part_b_variables = {variable.name: variable for variable in part_b_spec.variables}

    for variable_name in expected_variable_names:
        actual_variable = merged_dataset.variables[variable_name]
        actual_snapshot = VariableSpec(
            name=variable_name,
            dtype=str(actual_variable.dtype),
            dimensions=tuple(actual_variable.dimensions),
            attributes={
                attribute_name: _normalize_value(getattr(actual_variable, attribute_name))
                for attribute_name in actual_variable.ncattrs()
            },
            values=_normalize_value(actual_variable[:]),
        )

        acceptable_specs = []
        if variable_name in part_a_variables:
            acceptable_specs.append(part_a_variables[variable_name])
        if variable_name in part_b_variables:
            acceptable_specs.append(part_b_variables[variable_name])

        if not any(_variable_specs_match(actual_snapshot, spec) for spec in acceptable_specs):
            raise AssertionError(
                f"variable {variable_name!r} does not match any acceptable source variable"
            )


def run_stress_test(
    client: WindborneClient,
    *,
    iterations: int,
    seed: int = 0,
    name_prefix: str = "stress-case",
    artifacts_dir: str | Path | None = None,
    save_success_artifacts: bool = False,
    progress: bool = True,
) -> StressSummary:
    rng = random.Random(seed)
    artifact_root = Path(artifacts_dir) if artifacts_dir is not None else None

    for index in range(iterations):
        case = create_random_stress_case(rng, f"{name_prefix}-{index}")
        merged_bytes: bytes | None = None

        if progress:
            print(
                f"[{index + 1}/{iterations}] running {case.name} "
                f"(seed={seed}, part_a_vars={len(case.part_a_spec.variables)}, "
                f"part_b_vars={len(case.part_b_spec.variables)})"
            )
        try:
            client.upload(
                "part_a",
                name=case.name,
                dataset_name=f"{case.name}-part-a",
                netcdf_bytes=case.part_a_bytes,
            )
            client.upload(
                "part_b",
                name=case.name,
                dataset_name=f"{case.name}-part-b",
                netcdf_bytes=case.part_b_bytes,
            )

            merged_response = client.read_response(case.name)
            merged_bytes = merged_response.body
            merged_dataset = WindborneClient._dataset_from_response(merged_response)
            try:
                verify_merged_dataset(merged_dataset, case.part_a_spec, case.part_b_spec)
            finally:
                merged_dataset.close()
            if artifact_root is not None and save_success_artifacts:
                save_case_artifacts(
                    artifact_root,
                    case=case,
                    index=index,
                    seed=seed,
                    merged_bytes=merged_bytes,
                    status="passed",
                )
            if progress:
                print(f"  passed {case.name}")
        except Exception as exc:
            if artifact_root is not None:
                save_case_artifacts(
                    artifact_root,
                    case=case,
                    index=index,
                    seed=seed,
                    merged_bytes=merged_bytes,
                    status="failed",
                    error_message=str(exc),
                )
            raise RuntimeError(
                f"stress case failed: index={index}, name={case.name!r}, seed={seed}"
            ) from exc

    return StressSummary(seed=seed, total_cases=iterations, passed_cases=iterations)


def build_stress_arg_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Stress-test the Windborne NetCDF merge server.")
    parser.add_argument("--base-url", default="http://127.0.0.1:8000")
    parser.add_argument("--iterations", type=int, default=100)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--name-prefix", default="stress-case")
    parser.add_argument("--artifacts-dir", default="stress-artifacts")
    parser.add_argument("--save-success-artifacts", action="store_true")
    parser.add_argument("--quiet", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> None:
    parser = build_stress_arg_parser()
    args = parser.parse_args(argv)

    client = WindborneClient(args.base_url)
    summary = run_stress_test(
        client,
        iterations=args.iterations,
        seed=args.seed,
        name_prefix=args.name_prefix,
        artifacts_dir=args.artifacts_dir,
        save_success_artifacts=args.save_success_artifacts,
        progress=not args.quiet,
    )
    print(
        f"Stress test passed: {summary.passed_cases}/{summary.total_cases} cases "
        f"(seed={summary.seed}, base_url={args.base_url})"
    )


def _random_dimensions(rng: random.Random) -> dict[str, int]:
    dimensions = {"lat": rng.randint(1, 4), "lon": rng.randint(1, 4)}
    if rng.random() < 0.5:
        dimensions["level"] = rng.randint(1, 3)
    return dimensions


def _random_global_attributes(
    rng: random.Random,
    *,
    attribute_names: tuple[str, ...],
    count: int,
    defaults: dict[str, ScalarValue],
    reserved_names: set[str] | None = None,
) -> dict[str, ScalarValue]:
    attributes = dict(defaults)
    available_names = list(attribute_names)
    rng.shuffle(available_names)
    reserved_names = reserved_names or set()

    for attribute_name in available_names:
        if len(attributes) >= count:
            break

        if attribute_name in attributes or attribute_name in reserved_names:
            continue

        attributes[attribute_name] = _random_scalar_value(rng)

    return attributes


def _random_variable_specs(
    rng: random.Random,
    *,
    dimensions: dict[str, int],
    variable_names: tuple[str, ...],
    count: int,
    reserved_names: set[str] | None = None,
) -> list[VariableSpec]:
    variables: list[VariableSpec] = []
    chosen_names: set[str] = set()
    available_names = list(variable_names)
    rng.shuffle(available_names)
    reserved_names = reserved_names or set()

    while len(variables) < count and available_names:
        variable_name = available_names.pop()
        if variable_name in chosen_names or variable_name in reserved_names:
            continue

        variable_dimensions = _random_variable_dimensions(rng, tuple(dimensions.keys()))
        dtype = rng.choice(("f4", "i4"))
        variables.append(
            VariableSpec(
                name=variable_name,
                dtype=dtype,
                dimensions=variable_dimensions,
                attributes={"units": _random_units(rng)},
                values=_random_values(
                    rng,
                    shape=tuple(dimensions[name] for name in variable_dimensions),
                    dtype=dtype,
                ),
            )
        )
        chosen_names.add(variable_name)

    return variables


def _random_variable_dimensions(
    rng: random.Random,
    dimension_names: tuple[str, ...],
) -> tuple[str, ...]:
    if len(dimension_names) == 2 or rng.random() < 0.6:
        return tuple(dimension_names[:2])
    if rng.random() < 0.5:
        return (dimension_names[0],)
    return tuple(dimension_names)


def _random_values(rng: random.Random, *, shape: tuple[int, ...], dtype: str) -> Any:
    if len(shape) == 1:
        return [_random_scalar_for_dtype(rng, dtype) for _ in range(shape[0])]

    return [
        _random_values(rng, shape=shape[1:], dtype=dtype)
        for _ in range(shape[0])
    ]


def _random_scalar_for_dtype(rng: random.Random, dtype: str) -> ScalarValue:
    if dtype == "i4":
        return rng.randint(-500, 500)
    return round(rng.uniform(-500.0, 500.0), 3)


def _random_scalar_value(rng: random.Random) -> ScalarValue:
    kind = rng.choice(("string", "int", "float"))
    if kind == "string":
        return f"value-{rng.randint(0, 9999)}"
    if kind == "int":
        return rng.randint(-1000, 1000)
    return round(rng.uniform(-1000.0, 1000.0), 3)


def _random_units(rng: random.Random) -> str:
    return rng.choice(("K", "%", "Pa", "m/s", "mm", "1"))


def _assert_attributes_match_requirement(
    *,
    actual_attributes: dict[str, ScalarValue],
    part_a_attributes: dict[str, ScalarValue],
    part_b_attributes: dict[str, ScalarValue],
    label: str,
) -> None:
    expected_names = set(part_a_attributes) | set(part_b_attributes)
    actual_names = set(actual_attributes)
    if actual_names != expected_names:
        raise AssertionError(
            f"{label} mismatch: expected {sorted(expected_names)}, got {sorted(actual_names)}"
        )

    for attribute_name in expected_names:
        acceptable_values = []
        if attribute_name in part_a_attributes:
            acceptable_values.append(part_a_attributes[attribute_name])
        if attribute_name in part_b_attributes:
            acceptable_values.append(part_b_attributes[attribute_name])

        actual_value = actual_attributes[attribute_name]
        if not any(_values_match(actual_value, value) for value in acceptable_values):
            raise AssertionError(
                f"{label} mismatch for {attribute_name!r}: "
                f"expected one of {acceptable_values!r}, got {actual_value!r}"
            )


def _variable_specs_match(actual: VariableSpec, expected: VariableSpec) -> bool:
    return (
        actual.name == expected.name
        and actual.dtype == _normalized_dtype(expected.dtype)
        and actual.dimensions == expected.dimensions
        and actual.attributes == expected.attributes
        and _values_match(actual.values, expected.values)
    )


def _normalize_value(value: Any) -> Any:
    if hasattr(value, "tolist"):
        value = value.tolist()
    if hasattr(value, "item"):
        try:
            value = value.item()
        except ValueError:
            pass
    if isinstance(value, list):
        return [_normalize_value(item) for item in value]
    if isinstance(value, tuple):
        return tuple(_normalize_value(item) for item in value)
    return value


def _values_match(actual: Any, expected: Any, *, tolerance: float = 1e-4) -> bool:
    actual = _normalize_value(actual)
    expected = _normalize_value(expected)

    if isinstance(actual, list) and isinstance(expected, list):
        return len(actual) == len(expected) and all(
            _values_match(actual_item, expected_item, tolerance=tolerance)
            for actual_item, expected_item in zip(actual, expected, strict=True)
        )

    if isinstance(actual, tuple) and isinstance(expected, tuple):
        return len(actual) == len(expected) and all(
            _values_match(actual_item, expected_item, tolerance=tolerance)
            for actual_item, expected_item in zip(actual, expected, strict=True)
        )

    if isinstance(actual, float) or isinstance(expected, float):
        return abs(float(actual) - float(expected)) <= tolerance #type: ignore

    return actual == expected


def _normalized_dtype(dtype: str) -> str:
    if dtype == "f4":
        return "float32"
    if dtype == "i4":
        return "int32"
    return dtype


def save_case_artifacts(
    artifact_root: Path,
    *,
    case: StressCase,
    index: int,
    seed: int,
    merged_bytes: bytes | None,
    status: str,
    error_message: str | None = None,
) -> None:
    case_dir = artifact_root / f"{index:04d}-{case.name}"
    case_dir.mkdir(parents=True, exist_ok=True)

    (case_dir / "part_a.nc").write_bytes(case.part_a_bytes)
    (case_dir / "part_b.nc").write_bytes(case.part_b_bytes)
    if merged_bytes is not None:
        (case_dir / "merged.nc").write_bytes(merged_bytes)

    manifest = {
        "index": index,
        "name": case.name,
        "seed": seed,
        "status": status,
        "error_message": error_message,
        "part_a_spec": _spec_to_json(case.part_a_spec),
        "part_b_spec": _spec_to_json(case.part_b_spec),
        "merged_spec": _spec_from_netcdf_bytes(merged_bytes) if merged_bytes is not None else None,
        "files": {
            "part_a": "part_a.nc",
            "part_b": "part_b.nc",
            "merged": "merged.nc" if merged_bytes is not None else None,
        },
    }
    (case_dir / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")


def _spec_to_json(spec: DatasetSpec) -> dict[str, Any]:
    return {
        "dimensions": spec.dimensions,
        "global_attributes": spec.global_attributes,
        "variables": [
            {
                "name": variable.name,
                "dtype": variable.dtype,
                "dimensions": variable.dimensions,
                "attributes": variable.attributes,
                "values": variable.values,
            }
            for variable in spec.variables
        ],
    }


def _spec_from_netcdf_bytes(netcdf_bytes: bytes) -> dict[str, Any]:
    dataset = Dataset("artifact.nc", mode="r", memory=netcdf_bytes)

    try:
        return {
            "dimensions": {
                name: len(dimension) for name, dimension in dataset.dimensions.items()
            },
            "global_attributes": {
                attribute_name: _normalize_value(getattr(dataset, attribute_name))
                for attribute_name in dataset.ncattrs()
            },
            "variables": [
                {
                    "name": variable_name,
                    "dtype": str(variable.dtype),
                    "dimensions": list(variable.dimensions),
                    "attributes": {
                        attribute_name: _normalize_value(getattr(variable, attribute_name))
                        for attribute_name in variable.ncattrs()
                    },
                    "values": _normalize_value(variable[:]),
                }
                for variable_name, variable in dataset.variables.items()
            ],
        }
    finally:
        dataset.close()
