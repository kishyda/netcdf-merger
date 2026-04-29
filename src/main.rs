use std::env;
use wind_merge::server::server;
use wind_merge::worker::worker;

#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = env::args().nth(1);

    match mode.as_deref() {
        None | Some("server") | Some("--server") => server::run_server().await?,
        Some("worker") | Some("--worker") => worker::run_worker()?,
        Some(mode) => {
            return Err(format!(
                "unknown mode {mode:?}; expected server, worker, or --help"
            )
            .into());
        }
    }

    Ok(())
}
