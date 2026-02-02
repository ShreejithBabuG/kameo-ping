use kameo::prelude::*;
use kameo::remote;  
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;
use std::time::Instant;

// We need to define the same actor structure for type safety
#[derive(Actor)]
pub struct PingActor {
    ping_count: u64,
}

impl RemoteActor for PingActor {
    const REMOTE_ID: &'static str = "ping_pong_app::PingActor";
}

// Define the Ping message (must match server)
#[derive(Serialize, Deserialize, Clone)]
pub struct Ping {
    message: String,
    sequence: u64,
}

// Define the Pong response (must match server)
#[derive(Serialize, Deserialize, Reply)]
pub struct Pong {
    message: String,
    sequence: u64,
    total_pings: u64,
}

// We need to define the message handler even though we won't use it locally
// This is required for the type system
#[remote_message("a1b2c3d4-e5f6-7890-abcd-ef1234567890")]  
impl Message<Ping> for PingActor {
    type Reply = Pong;

    async fn handle(&mut self, _msg: Ping, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // This won't be called on the client side
        unreachable!("This handler should not be called on the client")
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

    info!(" Starting Ping Client...");

    // Bootstrap the distributed actor system
    let peer_id = remote::bootstrap()?;
    info!(" Client Peer ID: {}", peer_id);

    // Give the network a moment to discover peers
    info!(" Searching for remote PingActor...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Look up the remote PingActor
    let remote_actor = loop {
        match RemoteActorRef::<PingActor>::lookup("ping_actor").await? {
            Some(actor) => {
                info!(" Found remote PingActor!");
                break actor;
            }
            None => {
                warn!("⏳ PingActor not found yet, retrying in 2 seconds...");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    };

    info!(" Connected to remote PingActor");
    info!("");
    info!(" Starting ping-pong sequence...");
    info!("");
    
    // Start timing
    let start = Instant::now();

    // Send 100 ping messages
    for i in 1..=100 {
        let ping = Ping {
            message: format!("Hello from client, ping #{}", i),
            sequence: i,
        };

        info!(" Sending PING #{}: '{}'", i, ping.message);

        match remote_actor.ask(&ping).await {
            Ok(pong) => {
                info!(
                    " Received PONG #{}: '{}' (server has received {} total pings)",
                    pong.sequence, pong.message, pong.total_pings
                );
            }
            Err(e) => {
                error!(" Failed to receive pong: {}", e);
            }
        }

        info!("");

        // Wait a bit before sending the next ping
        if i < 100 {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    
    // Calculate elapsed time
    let duration = start.elapsed();

    info!(" All pings sent and responses received!");
    info!("  Total time: {:?}", duration);
    info!(" Average round-trip time: {:?}", duration / 100);
    info!(" Client shutting down...");
    Ok(())
}
