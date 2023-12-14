// server_setup.rs

use colored::Colorize;
use notify::{RecursiveMode, Watcher, Config, FsEventWatcher};
use reqwest;
use dialoguer::{Select, Confirm};
use serde_json::Value;
use tokio::signal::unix::Signal;
use watchexec::Watchexec;
use std::{fs::{self, File}, io::{Write, BufReader, BufRead, self}, path::{PathBuf, Path}, process::{Command, Stdio}, thread, borrow::Borrow, sync::{mpsc, Arc}, time::Duration};

pub struct ServerManager {
    project_name: String,
    folder_name: PathBuf,
}

impl ServerManager {
    pub async fn new(project_name: &str, folder_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let folder_path = PathBuf::from(folder_name);
        if !folder_path.exists() {
            fs::create_dir_all(&folder_path)?;
        }
        Ok(Self {
            project_name: project_name.to_owned(),
            folder_name: folder_path,
        })
    }

    pub async fn setup_and_run_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let versions = self.fetch_versions().await?;
        let selected_version = self.select_option(&versions, "Choose server version")?;
        let builds = self.fetch_builds(&selected_version).await?;
        let selected_build = builds.last().ok_or("No builds found")?.to_string();

        let download_url = self.construct_download_url(&selected_version, &selected_build);
        let file_path = self.folder_name.join("server.jar");

        self.download_server(&download_url, &file_path).await?;

        if Confirm::new().with_prompt("Do you accept the EULA?").interact()? {
            self.accept_eula()?;
        } else {
            println!("You need to accept the EULA to run the server.");
            return Ok(());
        }

        self.run_server(&file_path).await?;
        self.cleanup_server().await?;

        Ok(())
    }

   pub  async fn fetch_versions(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = format!("https://papermc.io/api/v2/projects/{}", self.project_name);
        let response = reqwest::get(&url).await?.text().await?;
        let data: Value = serde_json::from_str(&response)?;
        let versions = data["versions"].as_array().ok_or("No versions found")?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

        Ok(versions)
    }

    pub async fn fetch_builds(&self, version: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = format!("https://papermc.io/api/v2/projects/{}/versions/{}", self.project_name, version);
        let response = reqwest::get(&url).await?.text().await?;
        let data: Value = serde_json::from_str(&response)?;
        let builds = data["builds"].as_array().ok_or("No builds found")?
            .iter()
            .map(|b| b.to_string())
            .collect();

        Ok(builds)
    }

    fn select_option(&self, options: &[String], prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let selection = Select::new()
            .with_prompt(prompt)
            .default(0)
            .items(&options)
            .interact()?;

        Ok(options[selection].clone())
    }

   pub  fn construct_download_url(&self, version: &str, build: &str) -> String {
        format!(
            "https://papermc.io/api/v2/projects/{}/versions/{}/builds/{}/downloads/paper-{}-{}.jar",
            self.project_name, version, build, version, build
        )
    }

    pub async fn download_server(&self, url: &str, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let response = reqwest::get(url).await?;
        let mut file = File::create(file_path)?;
        let content = response.bytes().await?;
        file.write_all(&content)?;
        Ok(())
    }

   pub  fn accept_eula(&self) -> Result<(), Box<dyn std::error::Error>> {
        let eula_path = self.folder_name.join("eula.txt");
        let mut eula_file = File::create(eula_path)?;
        writeln!(eula_file, "eula=true")?;
        Ok(())
    }

    pub async fn run_server(&self, file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut child = Command::new("java")
            .current_dir(&self.folder_name)
            .args(&["-jar", "server.jar", "nogui"])
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        let mut stdin = child.stdin.take().expect("Failed to open child stdin");
        let stdout = child.stdout.take().expect("Failed to open child stdout");

         let reader = BufReader::new(stdout);

        // Spawn a new thread to handle the output
        thread::spawn(move || {
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        println!("{} {}", "|MINECRAFT|".green(), line.white());
                    }
                    Err(e) => eprintln!("Error reading line: {}", e),
                }
            }
        });


        let wx = Watchexec::new(|mut action| {
            // print any events
            for event in action.events.iter() {
                println!("{} {}", "|CARTRIDGE|".bright_purple(), event);
            }
    
            action
        })?;
    
    
        
    
        wx.main();
        


        let rtdin = io::stdin();
        for line in rtdin.lock().lines() {
            let line = line.expect("Failed to read line");
            if !line.starts_with('#') {
                if let Err(e) = stdin.write_all(line.as_bytes()) {
                    eprintln!("Failed to write to child stdin: {}", e);
                    break; // Break out of the loop if an error occurs
                }
                if let Err(e) = stdin.write_all(b"\n") {
                    eprintln!("Failed to write newline to child stdin: {}", e);
                    break; // Break out of the loop if an error occurs
                }
            }
            else {
                handle_command(&line)
            }
        }

        child.wait()?;
        Ok(())
    }

   pub async fn cleanup_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::remove_dir_all(&self.folder_name)?;
        Ok(())
    }
}

pub fn handle_command(command: &str) {
    match command {
        "#help" => {
            println!("Available commands:");
            println!("#help - Display this help message");
            println!("#watch - Watch a file and reload (NOT IMPLEMENTED)");
            // Add other commands here
        }
        // Handle other commands here
        _ => println!("Unknown command: {}", command),
    }
}

pub fn reload_server(child_stdin: &mut std::process::ChildStdin) {
    if let Err(e) = child_stdin.write_all(b"reload\n") {
        eprintln!("Failed to send reload command to server: {}", e);
    }
}