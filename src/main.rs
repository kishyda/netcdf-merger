use std::env;
use wind_merge::client::{self, load_client::LoadClientConfig};
use wind_merge::server::server;
use wind_merge::worker::worker;

#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = env::args().nth(1);

    match mode.as_deref() {
        None | Some("server") | Some("--server") => server::run_server().await?,
        Some("worker") | Some("--worker") => worker::run_worker()?,
        Some("client") | Some("load-client") => run_load_client(env::args().skip(2).collect())?,
        Some("help") | Some("--help") | Some("-h") => print_usage(),
        Some(mode) => {
            return Err(format!(
                "unknown mode {mode:?}; expected server, worker, client, or --help"
            )
            .into());
        }
    }

    Ok(())
}

fn run_load_client(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|arg| matches!(arg.as_str(), "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let config = parse_load_client_args(args)?;
    let summary = client::load_client::run(config)?;
    println!(
        "Rust load client complete: cases={}, parallelism={}, elapsed={:.3}s, throughput={:.2} cases/s ({:.2} requests/s)",
        summary.iterations,
        summary.parallelism,
        summary.elapsed.as_secs_f64(),
        summary.cases_per_second(),
        summary.requests_per_second(),
    );
    Ok(())
}

fn parse_load_client_args(args: Vec<String>) -> Result<LoadClientConfig, String> {
    let mut config = LoadClientConfig::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base-url" => {
                config.base_url = next_arg(&mut args, "--base-url")?;
            }
            "--iterations" => {
                config.iterations = next_arg(&mut args, "--iterations")?
                    .parse()
                    .map_err(|err| format!("invalid --iterations: {err}"))?;
            }
            "--parallelism" => {
                config.parallelism = next_arg(&mut args, "--parallelism")?
                    .parse()
                    .map_err(|err| format!("invalid --parallelism: {err}"))?;
            }
            "--name-prefix" => {
                config.name_prefix = next_arg(&mut args, "--name-prefix")?;
            }
            "--rows" => {
                config.rows = next_arg(&mut args, "--rows")?
                    .parse()
                    .map_err(|err| format!("invalid --rows: {err}"))?;
            }
            "--cols" => {
                config.cols = next_arg(&mut args, "--cols")?
                    .parse()
                    .map_err(|err| format!("invalid --cols: {err}"))?;
            }
            "--variables-per-part" => {
                config.variables_per_part = next_arg(&mut args, "--variables-per-part")?
                    .parse()
                    .map_err(|err| format!("invalid --variables-per-part: {err}"))?;
            }
            unknown => return Err(format!("unknown client argument {unknown:?}")),
        }
    }

    Ok(config)
}

fn next_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn print_usage() {
    println!(
        "Usage:\n  wind-merge [server]\n  wind-merge worker\n  wind-merge client [--base-url URL] [--iterations N] [--parallelism N] [--name-prefix PREFIX] [--rows N] [--cols N] [--variables-per-part N]"
    );
}
