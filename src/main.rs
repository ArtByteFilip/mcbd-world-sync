mod network;
mod config;
mod file_manager;

use anyhow::Result;
use notify::{Watcher, RecursiveMode, Event, RecommendedWatcher, Config as NotifyConfig};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use log::{info, error, warn, debug};
use std::fs;
use std::env;
use network::{SyncServer, SyncClient};
use std::path::PathBuf;
use config::Config as AppConfig;
use file_manager::{FileManager, FileInfo};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::SystemTime;

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
                match entry {
                    Ok(entry) => {
                        match entry.metadata() {
                            Ok(metadata) => {
                                if metadata.is_dir() {
                                    found_worlds = true;
                                    info!("Found world: {}", entry.path().display());
                                    // List contents of the world directory
                                    match fs::read_dir(entry.path()) {
                                        Ok(world_entries) => {
                                            for world_entry in world_entries {
                                                match world_entry {
                                                    Ok(world_entry) => {
                                                        debug!("  - {}", world_entry.path().display());
                                                    }
                                                    Err(e) => {
                                                        warn!("Could not read world entry: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                                error!("Access denied to world directory. Please run the program as administrator.");
                                            } else {
                                                warn!("Could not read world directory: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::PermissionDenied {
                                    error!("Access denied to world metadata. Please run the program as administrator.");
                                } else {
                                    warn!("Could not read metadata: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            error!("Access denied to directory entry. Please run the program as administrator.");
                        } else {
                            warn!("Could not read directory entry: {}", e);
                        }
                    }
                }
            }
            if !found_worlds {
                warn!("No Minecraft worlds found in the directory");
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                error!("Access denied to worlds directory. Please run the program as administrator.");
            } else {
                warn!("Could not read worlds directory: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger with debug level
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    
    info!("Starting Minecraft Bedrock World Sync");
    info!("Note: This program requires administrator privileges to access Minecraft files.");

    // Load configuration
    let config = AppConfig::load()?;
    info!("Configuration loaded");

    // Initialize file manager
    let file_manager = Arc::new(Mutex::new(FileManager::new(PathBuf::from(&config.paths.minecraft_worlds))));
    
    // Start sync server
    let server = SyncServer::new(config.server.port);
    let _file_manager_clone = file_manager.clone();
    
    tokio::spawn(async move {
        if let Err(e) = server.start().await {
            error!("Server error: {}", e);
        }
    });

    // Create a channel to receive the events
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default().with_poll_interval(Duration::from_secs(2)))?;

    // Try each possible path
    for path in get_minecraft_paths() {
        let worlds_path = Path::new(&path);
        info!("Checking path: {}", worlds_path.display());
        
        if worlds_path.exists() {
            info!("Found valid Minecraft directory: {}", worlds_path.display());
            
            // List worlds immediately
            list_worlds(worlds_path);

            // Initial scan of files
            let mut file_manager_guard = file_manager.lock().await;
            match file_manager_guard.scan_directory() {
                Ok(files) => {
                    info!("Found {} files to sync", files.len());
                }
                Err(e) => {
                    if e.to_string().contains("Access is denied") {
                        error!("Access denied during initial scan. Please run the program as administrator.");
                    } else {
                        error!("Error during initial scan: {}", e);
                    }
                    continue;
                }
            }
            drop(file_manager_guard);

            info!("Watching directory for changes: {}", worlds_path.display());
            if let Err(e) = watcher.watch(worlds_path, RecursiveMode::Recursive) {
                if e.to_string().contains("Access is denied") {
                    error!("Access denied to watch directory. Please run the program as administrator.");
                } else {
                    error!("Failed to watch directory: {}", e);
                }
                continue;
            }

            // Process events
            loop {
                match rx.recv() {
                    Ok(Ok(Event { kind, paths, .. })) => {
                        for path in paths {
                            info!("Change detected: {:?} - {:?}", kind, path);
                            
                            // Update file info
                            let mut file_manager_guard = file_manager.lock().await;
                            match fs::metadata(&path) {
                                Ok(metadata) => {
                                    match path.strip_prefix(worlds_path) {
                                        Ok(relative_path) => {
                                            match file_manager_guard.calculate_file_hash(&path) {
                                                Ok(hash) => {
                                                    let file_info = FileInfo {
                                                        path: relative_path.to_path_buf(),
                                                        last_modified: metadata.modified()?,
                                                        size: metadata.len(),
                                                        hash,
                                                    };
                                                    file_manager_guard.update_file_info(relative_path.to_path_buf(), file_info);
                                                }
                                                Err(e) => {
                                                    if e.to_string().contains("Access is denied") {
                                                        error!("Access denied to calculate file hash. Please run the program as administrator.");
                                                    } else {
                                                        error!("Failed to calculate file hash: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => error!("Failed to get relative path: {}", e),
                                    }
                                }
                                Err(e) => {
                                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                                        error!("Access denied to file metadata. Please run the program as administrator.");
                                    } else {
                                        error!("Failed to get file metadata: {}", e);
                                    }
                                }
                            }
                            drop(file_manager_guard);

                            // Send change to other devices
                            for device in &config.sync.devices {
                                let client = SyncClient::new(device.address.clone());
                                if let Err(e) = client.send_file_change(
                                    PathBuf::from(path.strip_prefix(worlds_path)?),
                                    format!("{:?}", kind)
                                ).await {
                                    error!("Failed to send change to {}: {}", device.name, e);
                                }
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
