mod network;

use anyhow::{Result, Context};
use notify::{Watcher, RecursiveMode, Event, RecommendedWatcher, Config};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use log::{info, error, warn, debug};
use std::fs;
use std::env;
use network::{SyncServer, SyncClient};
use std::path::PathBuf;

fn get_username() -> String {
    // Try different environment variables and methods to get the username
    if let Ok(username) = env::var("USERNAME") {
        return username;
    }
    if let Ok(username) = env::var("USER") {
        return username;
    }
    if let Ok(username) = env::var("USERPROFILE") {
        if let Some(name) = Path::new(&username).file_name() {
            if let Some(name_str) = name.to_str() {
                return name_str.to_string();
            }
        }
    }
    // Fallback to a default if nothing else works
    "unknown".to_string()
}

fn get_minecraft_paths() -> Vec<String> {
    let username = get_username();
    info!("Detected username: {}", username);
    
    vec![
        format!("C:\\Users\\{}\\AppData\\Local\\Packages\\Microsoft.MinecraftUWP_8wekyb3d8bbwe\\LocalState\\games\\com.mojang\\minecraftWorlds", username),
        format!("C:\\Users\\{}\\AppData\\Local\\Packages\\Microsoft.MinecraftUWP_8wekyb3d8bbwe\\LocalState\\games\\com.mojang\\development_behavior_packs", username)
    ]
}

fn list_worlds(path: &Path) {
    info!("Scanning for Minecraft worlds in: {}", path.display());
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut found_worlds = false;
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            found_worlds = true;
                            info!("Found world: {}", entry.path().display());
                            // List contents of the world directory
                            if let Ok(world_entries) = fs::read_dir(entry.path()) {
                                for world_entry in world_entries {
                                    if let Ok(world_entry) = world_entry {
                                        debug!("  - {}", world_entry.path().display());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !found_worlds {
                warn!("No Minecraft worlds found in the directory");
            }
        }
        Err(e) => warn!("Could not read worlds directory: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger with debug level
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    
    info!("Starting Minecraft Bedrock World Sync");
    info!("Current working directory: {:?}", env::current_dir()?);

    // Start sync server
    let server = SyncServer::new(8080);
    tokio::spawn(async move {
        if let Err(e) = server.start().await {
            error!("Server error: {}", e);
        }
    });

    // Create a channel to receive the events
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events
    let mut watcher = RecommendedWatcher::new(tx, Config::default().with_poll_interval(Duration::from_secs(2)))?;

    // Try each possible path
    for path in get_minecraft_paths() {
        let worlds_path = Path::new(&path);
        info!("Checking path: {}", worlds_path.display());
        
        if worlds_path.exists() {
            info!("Found valid Minecraft directory: {}", worlds_path.display());
            
            // List worlds immediately
            list_worlds(worlds_path);

            info!("Watching directory for changes: {}", worlds_path.display());
            if let Err(e) = watcher.watch(worlds_path, RecursiveMode::Recursive) {
                error!("Failed to watch directory: {}", e);
                continue;
            }

            // Process events
            loop {
                match rx.recv() {
                    Ok(Ok(Event { kind, paths, .. })) => {
                        for path in paths {
                            info!("Change detected: {:?} - {:?}", kind, path);
                            
                            // Send change to other devices
                            let client = SyncClient::new("127.0.0.1:8080".to_string());
                            if let Err(e) = client.send_file_change(
                                PathBuf::from(path),
                                format!("{:?}", kind)
                            ).await {
                                error!("Failed to send change: {}", e);
                            }

                            // List worlds again after change
                            list_worlds(worlds_path);
                        }
                    }
                    Ok(Err(e)) => error!("Watch error: {:?}", e),
                    Err(e) => error!("Channel error: {:?}", e),
                }
            }
        } else {
            warn!("Directory does not exist: {}", worlds_path.display());
        }
    }

    warn!("No valid Minecraft directories found. Please make sure Minecraft Bedrock Edition is installed.");
    Ok(())
}
