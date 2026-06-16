mod connectivity_check;
mod diagnostics;
mod dns_check;
mod engine_runner;
mod engine_supervisor;
mod logging;
mod profile_runner;
mod recovery;
mod revert;
mod service;
mod service_api;
mod state_snapshot;

use anyhow::Result;
use service_api::ServiceApi;

fn main() -> Result<()> {
    let mut api = ServiceApi::new(std::env::current_dir()?);
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("status") => println!("{}", serde_json::to_string_pretty(&api.status()?)?),
        Some("enable") => {
            let profiles = args.iter().skip(2).cloned().collect();
            println!("{}", serde_json::to_string_pretty(&api.enable(profiles)?)?);
        }
        Some("disable") => println!("{}", serde_json::to_string_pretty(&api.disable_all()?)?),
        Some("diagnostics") => {
            println!("{}", serde_json::to_string_pretty(&api.run_diagnostics()?)?)
        }
        Some("emergency-disable") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&api.emergency_disable()?)?
            )
        }
        Some("install") => println!("Zapret Manager mock service install requested."),
        Some("uninstall") => println!("Zapret Manager mock service uninstall requested."),
        _ => {
            println!("Zapret Manager Service mock runner");
            println!("commands: status | enable <profiles...> | disable | diagnostics | emergency-disable");
        }
    }
    Ok(())
}
