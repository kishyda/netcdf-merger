from __future__ import annotations

import argparse

from netCDF4 import Dataset

from .api import WindborneClient
from .display import describe_dataset, dump_dataset
from .samples import create_expected_output_netcdf_bytes
from .stress import format_stress_profile, run_stress_test, stress_profile_recommendations


def run_demo(base_url: str) -> None:
    client = WindborneClient(base_url)
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


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Windborne NetCDF client tools")
    parser.add_argument("--base-url", default="http://127.0.0.1:8000")
    parser.add_argument("--stress", action="store_true")
    parser.add_argument("--iterations", type=int, default=1000)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--name-prefix", default="stress-case")
    parser.add_argument("--parallelism", type=int)
    parser.add_argument("--artifacts-dir", default="stress-artifacts")
    parser.add_argument("--save-success-artifacts", action="store_true")
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args(argv)

    if args.stress:
        summary = run_stress_test(
            WindborneClient(args.base_url),
            iterations=args.iterations,
            seed=args.seed,
            name_prefix=args.name_prefix,
            parallelism=args.parallelism,
            artifacts_dir=args.artifacts_dir,
            save_success_artifacts=args.save_success_artifacts,
            progress=not args.quiet,
        )
        print(
            f"Stress test passed: {summary.passed_cases}/{summary.total_cases} cases "
            f"(seed={summary.seed})"
        )
        print(f"Profile: {format_stress_profile(summary)}")
        print("Useful next measurements:")
        for recommendation in stress_profile_recommendations():
            print(f"- {recommendation}")
        return

    run_demo(args.base_url)


if __name__ == "__main__":
    main()
