from __future__ import annotations

from netCDF4 import Dataset


def describe_dataset(dataset: Dataset) -> str:
    dimensions = ", ".join(f"{name}={len(dim)}" for name, dim in dataset.dimensions.items())
    variables = ", ".join(dataset.variables.keys())
    global_attributes = ", ".join(dataset.ncattrs())
    title = getattr(dataset, "title", "<missing>")
    return f"title={title}; dims=[{dimensions}]; vars=[{variables}]; attrs=[{global_attributes}]"


def dump_dataset(dataset: Dataset) -> str:
    lines: list[str] = []

    lines.append("Global attributes:")
    for attr_name in dataset.ncattrs():
        lines.append(f"  {attr_name} = {getattr(dataset, attr_name)!r}")

    lines.append("Dimensions:")
    for dim_name, dim in dataset.dimensions.items():
        lines.append(f"  {dim_name} = {len(dim)}")

    lines.append("Variables:")
    for var_name, variable in dataset.variables.items():
        lines.append(f"  {var_name}")
        lines.append(f"    dims = {variable.dimensions}")
        lines.append(f"    shape = {variable.shape}")

        for attr_name in variable.ncattrs():
            lines.append(f"    @{attr_name} = {getattr(variable, attr_name)!r}")

        lines.append(f"    values = {variable[:].tolist()}")

    return "\n".join(lines)
