# windborne-oa

Rust server plus Python client for accepting two NetCDF uploads, merging them in memory, and returning the combined NetCDF.

**What This Repo Contains**
- [src](src): Rocket server and NetCDF merge logic
- [tests](tests): Rust integration tests
- [client/src/windborne_client](client/src/windborne_client): Python client package and stress harness
- [client/src/tests](client/src/tests): Python tests
- [vendor/netcdf](vendor/netcdf): patched copy of the `netcdf` crate

**Architecture**
- The HTTP server is built in [lib.rs](src/lib.rs) and launched from [main.rs](src/main.rs).
- Request handlers live in [routes.rs](src/routes.rs).
- Shared in-memory state lives in [state.rs](src/state.rs).
- Merge logic lives in [netcdf_operations.rs](src/netcdf_operations.rs).
- Error helpers live in [helpers.rs](src/helpers.rs).

**Server Data Flow**
1. `POST /part_a?name=...` or `POST /part_b?name=...` reads the request body into memory.
2. The body is validated with `netcdf::open_mem(...)` to ensure it is readable NetCDF.
3. The raw bytes are stored in memory under that `name`.
4. `GET /read?name=...` loads the two stored byte blobs, merges them into a fresh in-memory NetCDF, and returns the merged bytes.
5. The merged bytes are cached in memory per `name`.

**State Model**
- `AppState` is a `DashMap<String, DataSet>` in [state.rs](src/state.rs).
- Each `DataSet` holds:
  - `part_a_data`
  - `part_b_data`
  - `merged_data`
  - `version`
- `version` is incremented on every upload.
- `read_file()` snapshots the current entry, merges outside the map guard, then only stores the result if the version is unchanged. This avoids stale cache reinsertion after concurrent uploads.

**Merge Semantics**
- Dimensions: union by name. Existing dimensions are reused.
- Global attributes: union by name. If the same attribute appears in both parts, the first one encountered wins.
- Variables: union by name. If the same variable appears in both parts, the first one encountered wins.
- Variable attributes: copied from the winning variable definition.
- Variable data: copied once per variable name, from the winning source only.

In the current server flow, the merge order is `[part_a, part_b]`, so `part_a` wins duplicate conflicts.

**Why The `netcdf` Crate Is Vendored**
- The upstream safe `netcdf` crate does not expose a way to:
  - create a new in-memory NetCDF file mutate it, and get the final serialized bytes back without persisting the file to disk
- This repo patches the crate in [vendor/netcdf](vendor/netcdf) and overrides crates.io via [Cargo.toml](Cargo.toml).

The important patched APIs are:
- `netcdf::create_mem(initial_size)`
- `FileMut::close_to_bytes()`

Implementation detail:
- The patch uses `nc_create_mem` and `nc_close_memio` underneath.
- The in-memory create path uses `NC_NETCDF4`

**No-Disk Design**
- The application-level NetCDF flow is intended to stay in memory:
  - uploads are stored as `Vec<u8>`
  - merges use `open_mem(...)`
  - merged output uses `create_mem(...)` and `close_to_bytes()`
- This means the NetCDF payloads never touch the disk.

Important nuance:
- The process may still touch the filesystem for unrelated reasons:
  - runtime/loader activity
  - OpenSSL config
  - timezone files
  - libnetcdf config probing

**Running The Server**
```bash
cargo run
```

The Rocket app mounts:
- `POST /part_a?name=...`
- `POST /part_b?name=...`
- `GET /read?name=...`

**Running Rust Tests**
```bash
cargo test
```

The most important Rust tests are:
- [tests/routes_test.rs](tests/routes_test.rs): endpoint behavior
- [tests/netcdf_operations_test.rs](tests/netcdf_operations_test.rs): merge semantics and regressions
- [tests/helpers_test.rs](tests/helpers_test.rs): error helpers

**Python Client**
- The client package is in [client/src/windborne_client](client/src/windborne_client).
- [api.py](client/src/windborne_client/api.py): HTTP client
- [samples.py](client/src/windborne_client/samples.py): simple sample NetCDF builders
- [display.py](client/src/windborne_client/display.py): readable dataset dumps
- [stress.py](client/src/windborne_client/stress.py): randomized stress harness
- [__main__.py](client/src/windborne_client/__main__.py): CLI entrypoint

**Running The Client**
From [client](client):

```bash
uv run python -m src.windborne_client
```

That runs the simple demo flow.

**Running The Stress Harness**
From [client](client):

```bash
uv run python -m src.windborne_client --stress --iterations 100 --artifacts-dir stress-artifacts --save-success-artifacts
```

Useful flags:
- `--iterations`
- `--seed`
- `--name-prefix`
- `--artifacts-dir`
- `--save-success-artifacts`
- `--quiet`

**Stress Artifacts**
- The stress harness writes one directory per case under the artifact root.
- Each case directory contains:
  - `part_a.nc`
  - `part_b.nc`
  - `merged.nc` if the server returned one
  - `manifest.json`

`manifest.json` includes:
- case index/name/seed/status
- any error message
- `part_a_spec`
- `part_b_spec`
- `merged_spec`

This makes it easy to inspect the exact inputs and merged output for a failing or surprising case.

**Running Python Tests**
From [client](client):

```bash
PYTHONPYCACHEPREFIX=../.pycache .venv/bin/python -m unittest discover -s src/tests -p 'test_*.py'
```

**Current Limitations**
- Stored datasets and merged cache entries never expire.
- There is no memory cap or eviction policy.
- The API returns simple `"Success"` strings for uploads instead of structured JSON.
- The vendored `netcdf` crate is essential to the design; upgrading dependencies here is not a trivial `cargo update`.

**Parallel Request Handling**
Correctly supporting parallel requests for NetCDF operations involves several non-obvious challenges:
- Thread-safety limitations in NetCDF-C and HDF5
- Internal library file-name collisions for in-memory VFS
- Asynchronous runtime integration (blocking vs. non-blocking tasks)
- Preventing redundant computation (thundering herd)

Detailed design considerations for implementing these fixes can be found in [PARALLEL_REQUESTS.md](PARALLEL_REQUESTS.md).

**Most Relevant Files**
- [src/routes.rs](src/routes.rs)
- [src/netcdf_operations.rs](src/netcdf_operations.rs)
- [src/state.rs](src/state.rs)
- [tests/netcdf_operations_test.rs](tests/netcdf_operations_test.rs)
- [client/src/windborne_client/stress.py](client/src/windborne_client/stress.py)
- [vendor/netcdf/src/file.rs](vendor/netcdf/src/file.rs)
