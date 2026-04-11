# Problem
- netcdf-c is not generally thread-safe. Unidata has explicitly noted shared global state across threads, so concurrent calls into the library, even for different files, cannot be assumed safe.

# My Solution
- Separate application state from native library access as much as possible. Uploaded files are stored only as in-memory byte buffers rather than long-lived NetCDF objects.
- When /read is called, snapshot the relevant input buffers and then invoke the NetCDF library only for the duration of the merge.
- Serialize all access to the NetCDF merge path with a single global mutex, so only one request enters the library at a time. This matches the practical reality that some HDF5 thread-safe configurations also serialize access internally with a global lock rather than providing true parallel execution.
- Treat each merge as one atomic operation. Even if individual library calls are serialized, allowing separate merge workflows to interleave between steps adds complexity without providing any real upside. Since the native library path must effectively be single-threaded anyway, the clean design is to keep non-NetCDF work concurrent and make the full merge operation exclusive.

# Possible Future Changes
- If merges severely bottle necks the performance, then the logical next step would be to move the NetCDF merge work into multiple worker processes. Each worker would perform one merge operation at a time, avoiding the danger of multi-threaded access to the library, while allowing merge throughput to scale across isolated workers.