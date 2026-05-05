use std::io;
use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub struct WorkerClient {
    _child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

impl WorkerClient {
    pub fn spawn() -> io::Result<Self> {
        let executable = worker_executable()?;
        let mut child = Command::new(executable)
            .arg("worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("worker stdin was not piped"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("worker stdout was not piped"))?;

        Ok(Self {
            _child: child,
            stdin,
            stdout,
        })
    }

    pub async fn merge(&mut self, part_a: &[u8], part_b: &[u8]) -> io::Result<Vec<u8>> {
        write_u64(&mut self.stdin, part_a.len() as u64).await?;
        write_u64(&mut self.stdin, part_b.len() as u64).await?;
        self.stdin.write_all(part_a).await?;
        self.stdin.write_all(part_b).await?;
        self.stdin.flush().await?;

        let merged_len = read_u64(&mut self.stdout).await? as usize;
        let mut merged = vec![0; merged_len];
        self.stdout.read_exact(&mut merged).await?;

        Ok(merged)
    }
}

fn worker_executable() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_wind-merge") {
        return Ok(path.into());
    }

    let current_exe = std::env::current_exe()?;
    if current_exe
        .file_stem()
        .is_some_and(|name| name == "wind-merge")
    {
        return Ok(current_exe);
    }

    if current_exe.parent().and_then(|path| path.file_name()) == Some("deps".as_ref())
        && let Some(debug_dir) = current_exe.parent().and_then(|path| path.parent())
    {
        return Ok(debug_dir.join(format!("wind-merge{}", std::env::consts::EXE_SUFFIX)));
    }

    Ok(current_exe.with_file_name(format!("wind-merge{}", std::env::consts::EXE_SUFFIX)))
}

async fn read_u64<R>(reader: &mut R) -> io::Result<u64>
where
    R: AsyncReadExt + Unpin,
{
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf).await?;
    Ok(u64::from_le_bytes(buf))
}

async fn write_u64<W>(writer: &mut W, value: u64) -> io::Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    writer.write_all(&value.to_le_bytes()).await
}
