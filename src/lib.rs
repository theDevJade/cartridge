use std::{borrow::Borrow, path::PathBuf};

use api_util::ServerManager;
use clap::Parser;
use tracing::log;
use cliclack::*;

pub mod utils;
pub mod api_util;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct ToolArgs {
    /// Print additional information (pass argument one to four times for increasing detail)
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

}

pub async fn start(args: ToolArgs) {
    log::info!("Starting tool with args: {:?}", args);
    setup().await;
}

async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    intro("cartridge")?;

    let path: String = input("Where should we create your project?")
        .placeholder("./cartridge")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Please enter a path.")
            } else if !input.starts_with("./") {
                Err("Please enter a relative path")
            } else {
                Ok(())
            }
        })
        .interact()?;

    let eula: bool = confirm("Do you accept the EULA?").initial_value(true).interact()?;
    
    let server_manager = ServerManager::new("paper", &path).await?;

    if eula {
        server_manager.accept_eula()?;
    }

    let mut spinner = spinner();
    spinner.start("Fetching Versions...");
    let mut versions = server_manager.fetch_versions().await?;
    versions.sort();
    spinner.stop("Fetched Versions.");

    
    cliclack::log::info(format!("Server Versions: {}", versions.join(", ")))?;
    let version: String = input("Choose a version")
    .placeholder("Not sure")
    .validate(move |input: &String| {
        if !versions.contains(&input) {
            Err("Please enter a valid version.")
        } else {
            Ok(())
        }
    })
    .interact()?;

    spinner.start("Fetching Builds...");
    let mut builds = server_manager.fetch_builds(&version).await?;
    builds.sort();
    spinner.stop("Fetched Builds.");

    
    cliclack::log::info(format!("Version Builds: {}", builds.join(", ")))?;
    let build: String = input("Choose a build")
    .placeholder("Not sure")
    .validate(move |input: &String| {
        if !builds.contains(&input) {
            Err("Please enter a valid version.")
        } else {
            Ok(())
        }
    })
    .interact()?;

    let mut spinner2  = Spinner::default();
    spinner2.start("Downloading Server...");
    let server_jar_path = format!("{}/server.jar", path);
    let server_jar  = PathBuf::from(&server_jar_path);
    let url = server_manager.construct_download_url(&version, &build);
    server_manager.download_server(&url, &server_jar).await?;
    spinner2.stop("Downloaded Server.");

    outro("Running Server")?;

    server_manager.run_server(&server_jar).await?;

    intro("Cleaning up")?;
    let mut endSpinner = Spinner::default();
    endSpinner.start("Cleaning up...");
    server_manager.cleanup_server().await?;
    endSpinner.stop("Cleaned up.");
    outro("Finished! Bye!");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_start() {
        assert_eq!(1, 1);
    }
}
