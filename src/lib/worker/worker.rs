// netcdf_worker.rs
//
// Worker side:
//
// Reads from stdin:
//
//   u64 part_a_len
//   u64 part_b_len
//   [part_a bytes]
//   [part_b bytes]
//
// Writes to stdout:
//
//   u64 merged_len
//   [merged bytes]
//
// Replace merge_netcdf() with your real NetCDF merge logic.

use std::io::{self, Read, Write};

use crate::worker::netcdf_operations;

pub fn run_worker() -> io::Result<()> {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        let Some(part_a_len) = read_next_u64(&mut stdin)? else {
            return Ok(());
        };
        let part_b_len = read_u64(&mut stdin)?;

        let mut part_a = vec![0u8; part_a_len as usize];
        let mut part_b = vec![0u8; part_b_len as usize];

        stdin.read_exact(&mut part_a)?;
        stdin.read_exact(&mut part_b)?;

        let merged = netcdf_operations::combine_netcdf_bytes(&[&part_a, &part_b])
            .map_err(|err| io::Error::other(err.message))?;

        write_u64(&mut stdout, merged.len() as u64)?;
        stdout.write_all(&merged)?;
        stdout.flush()?;
    }
}

fn read_next_u64<R: Read>(reader: &mut R) -> io::Result<Option<u64>> {
    let mut buf = [0u8; 8];
    match reader.read_exact(&mut buf) {
        Ok(()) => Ok(Some(u64::from_le_bytes(buf))),
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(err) => Err(err),
    }
}

fn read_u64<R: Read>(reader: &mut R) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn write_u64<W: Write>(writer: &mut W, value: u64) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}
