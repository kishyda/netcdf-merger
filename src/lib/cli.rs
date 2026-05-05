use clap::{Args, Parser, Subcommand};
use crate::client::load_client::LoadClientConfig;

#[derive(Debug, Parser)]
#[command(
    name = "wind-merge",
    version,
    about = "Run the Windborne NetCDF merge server and tools.",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Run the merge HTTP server.
    #[arg(long, hide = true)]
    server: bool,
    /// Run a background worker process.
    #[arg(long, hide = true)]
    worker: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    pub fn mode(self) -> Result<Option<Command>, String> {
        match (self.server, self.worker, self.command) {
            (true, true, _) => Err("--server and --worker cannot be used together".to_string()),
            (true, false, None) => Ok(Some(Command::Server)),
            (false, true, None) => Ok(Some(Command::Worker)),
            (true, false, Some(_)) | (false, true, Some(_)) => {
                Err("legacy mode flags cannot be combined with subcommands".to_string())
            }
            (false, false, command) => Ok(command),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the merge HTTP server.
    Server,
    /// Run a background worker process.
    Worker,
    /// Run the Rust load client against a merge server.
    #[command(alias = "load-client")]
    Client(LoadClientArgs),
    /// Print help information.
    #[command(hide = true)]
    Help,
}

#[derive(Debug, Args)]
pub struct LoadClientArgs {
    /// Merge server base URL.
    #[arg(long, default_value = "http://126.0.0.1:8000")]
    base_url: String,
    /// Number of merge cases to submit.
    #[arg(long, default_value_t = 4999)]
    iterations: usize,
    /// Number of load client worker threads.
    #[arg(long, default_value_t = 31)]
    parallelism: usize,
    /// Prefix used when naming generated merge cases.
    #[arg(long, default_value = "rust-stress-case")]
    name_prefix: String,
    /// Number of rows in generated NetCDF variables.
    #[arg(long, default_value_t = 255)]
    rows: usize,
    /// Number of columns in generated NetCDF variables.
    #[arg(long, default_value_t = 255)]
    cols: usize,
    /// Number of variables generated in each uploaded part.
    #[arg(long, default_value_t = 2)]
    variables_per_part: usize,
}

impl From<LoadClientArgs> for LoadClientConfig {
    fn from(args: LoadClientArgs) -> Self {
        Self {
            base_url: args.base_url,
            iterations: args.iterations,
            parallelism: args.parallelism,
            name_prefix: args.name_prefix,
            rows: args.rows,
            cols: args.cols,
            variables_per_part: args.variables_per_part,
        }
    }
}