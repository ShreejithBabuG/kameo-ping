use kameo::prelude::*;
use kameo::remote;
use serde::{Deserialize, Serialize};
use tracing::info;
use tracing_subscriber::EnvFilter;

// Define the Ping Actor that will receive ping messages
#[derive(Actor)]
pub struct PingActor {
    ping_count: u64,
}

impl RemoteActor for PingActor {
    const REMOTE_ID: &'static str = "ping_pong_app::PingActor";
}

// Define the Ping message
#[derive(Serialize, Deserialize, Clone)]
pub struct Ping {
    message: String,
    sequence: u64,
}

// Define the Pong response
#[derive(Serialize, Deserialize, Reply)]
pub struct Pong {
    message: String,
    sequence: u64,
    total_pings: u64,
}

// Implement the message handler for Ping
#[remote_message("a1b2c3d4-e5f6-7890-abcd-ef1234567890")]  
impl Message<Ping> for PingActor {
    type Reply = Pong;

    async fn handle(&mut self, msg: Ping, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.ping_count += 1;
        
        info!(
            " Received PING #{} with message: '{}'",
            msg.sequence, msg.message
        );
        
        // Send back a Pong response
        let pong = Pong {
            message: format!("Pong! Responding to: {}", msg.message),
            sequence: msg.sequence,
            total_pings: self.ping_count,
        };
        
        info!(" Sending PONG #{} (total pings received: {})", msg.sequence, self.ping_count);
        
        pong
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    info!(" Starting Ping Server...");

    // Bootstrap the distributed actor system
    let peer_id = remote::bootstrap()?;
    info!(" Server Peer ID: {}", peer_id);
    
    // Get the local address we're listening on
    info!(" Server is listening for connections...");
    info!(" To connect from another machine, use this peer's multiaddress");

    // Spawn the PingActor
    let ping_actor = PingActor::spawn(PingActor { ping_count: 0 });
    
    // Register it so other nodes can find it
    ping_actor.register("ping_actor").await?;
    
    info!(" PingActor registered and ready to receive pings!");
    info!(" Waiting for ping messages from clients...");
    info!("");
    info!("Press Ctrl+C to stop the server");

    // Keep the server running
    tokio::signal::ctrl_c().await?;
    
    info!(" Shutting down server...");
    
    Ok(())
}
