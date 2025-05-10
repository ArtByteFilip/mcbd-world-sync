use anyhow::Result;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use log::{info, error, warn};

#[derive(Debug, Serialize, Deserialize)]
pub enum SyncMessage {
    FileChange {
        path: PathBuf,
        change_type: String,
    },
    FileContent {
        path: PathBuf,
        content: Vec<u8>,
    },
    SyncRequest,
    SyncResponse,
}

pub struct SyncServer {
    port: u16,
}

impl SyncServer {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        info!("Sync server listening on port {}", self.port);

        loop {
            let (socket, addr) = listener.accept().await?;
            info!("New connection from {}", addr);
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(socket).await {
                    error!("Error handling connection from {}: {}", addr, e);
                }
            });
        }
    }

    async fn handle_connection(socket: TcpStream) -> Result<()> {
        let mut framed = Framed::new(socket, LengthDelimitedCodec::new());

        while let Some(msg) = framed.next().await {
            match msg {
                Ok(bytes) => {
                    if let Ok(message) = serde_json::from_slice::<SyncMessage>(&bytes) {
                        match message {
                            SyncMessage::FileChange { path, change_type } => {
                                info!("Received file change: {} - {}", path.display(), change_type);
                                // TODO: Handle file change
                            }
                            SyncMessage::FileContent { path, content } => {
                                info!("Received file content for: {}", path.display());
                                // TODO: Save file content
                            }
                            SyncMessage::SyncRequest => {
                                info!("Received sync request");
                                // TODO: Send current state
                            }
                            SyncMessage::SyncResponse => {
                                info!("Received sync response");
                                // TODO: Handle sync response
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

pub struct SyncClient {
    server_address: String,
}

impl SyncClient {
    pub fn new(server_address: String) -> Self {
        Self { server_address }
    }

    pub async fn connect(&self) -> Result<()> {
        let socket = TcpStream::connect(&self.server_address).await?;
        info!("Connected to sync server at {}", self.server_address);
        
        let mut framed = Framed::new(socket, LengthDelimitedCodec::new());

        // Send initial sync request
        let sync_request = SyncMessage::SyncRequest;
        let bytes = serde_json::to_vec(&sync_request)?;
        framed.send(bytes).await?;

        Ok(())
    }

    pub async fn send_file_change(&self, path: PathBuf, change_type: String) -> Result<()> {
        let socket = TcpStream::connect(&self.server_address).await?;
        let mut framed = Framed::new(socket, LengthDelimitedCodec::new());

        let message = SyncMessage::FileChange { path, change_type };
        let bytes = serde_json::to_vec(&message)?;
        framed.send(bytes).await?;

        Ok(())
    }
} 