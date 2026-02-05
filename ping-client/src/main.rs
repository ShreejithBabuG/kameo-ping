use kameo::prelude::*;
use kameo::remote;
use libp2p::{
    noise, tcp, yamux,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;
use clap::Parser;
use futures::StreamExt;

// Command-line arguments for server address
#[derive(Parser, Debug)]
#[command(name = "ping-client")]
struct Args {
    #[arg(short, long)]
    server: Option<String>,
}

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    kameo: remote::Behaviour,
}

#[derive(Actor)]
pub struct PingActor {
    ping_count: u64,
}

// REMOTE_ID must match server for actor discovery
impl RemoteActor for PingActor {
    const REMOTE_ID: &'static str = "ping_pong_app::PingActor";
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Ping {
    message: String,
    sequence: u64,
}

#[derive(Serialize, Deserialize, Reply)]
pub struct Pong {
    message: String,
    sequence: u64,
    total_pings: u64,
}

// UUID must match server
#[remote_message("a1b2c3d4-e5f6-7890-abcd-ef1234567890")]
impl Message<Ping> for PingActor {
    type Reply = Pong;

    async fn handle(&mut self, _msg: Ping, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        unreachable!("This handler should not be called on the client")
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    info!(" Starting Ping Client with custom swarm...");

    if let Some(server_addr) = args.server {
        info!(" Using custom swarm configuration for direct connection");
        info!(" Server address: {}", server_addr);
        
        let server_multiaddr: Multiaddr = server_addr.parse()?;
        
        // Build custom swarm with same configuration as server
        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(tcp::Config::default(), noise::Config::new, || yamux::Config::default())?
            .with_behaviour(|key| {
                let peer_id = key.public().to_peer_id();
                let messaging_config = remote::messaging::Config::default()
                    .with_request_timeout(Duration::from_secs(120));
                let kameo = remote::Behaviour::new(peer_id, messaging_config);
                Ok(MyBehaviour { kameo })
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(600)))
            .build();

        swarm.behaviour().kameo.init_global();

        info!(" Client Peer ID: {}", swarm.local_peer_id());

        // Listen on any available port
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        info!(" Dialing server at {}...", server_multiaddr);
        swarm.dial(server_multiaddr.clone())?;

        // Run swarm event loop in background task
        let swarm_handle = tokio::spawn(async move {
            loop {
                match swarm.select_next_some().await {
                    SwarmEvent::Behaviour(MyBehaviourEvent::Kameo(event)) => {
                        info!(" Kameo event: {:?}", event);
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        info!(" Connected to {} via {}", peer_id, endpoint.get_remote_address());
                        
                        // Add server peer to address book for Kademlia DHT
                        let remote_addr = endpoint.get_remote_address().clone();
                        swarm.add_peer_address(peer_id, remote_addr.clone());
                        info!(" Added server to swarm: {} at {}", peer_id, remote_addr);
                    }
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        warn!(" Connection to {} closed: {:?}", peer_id, cause);
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(" Listening on {}", address);
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        warn!(" Failed to connect to {:?}: {}", peer_id, error);
                    }
                    _ => {}
                }
            }
        });

        // Wait for DHT to synchronize routing tables
        info!("⏳ Waiting for DHT to stabilize (15 seconds)...");
        tokio::time::sleep(Duration::from_secs(15)).await;

        // Lookup remote PingActor via Kademlia DHT with retries
        info!("🔍 Searching for remote PingActor...");
        let mut retry_count = 0;
        let max_retries = 10;
        
        let remote_actor = loop {
            match RemoteActorRef::<PingActor>::lookup("ping_actor").await? {
                Some(actor) => {
                    info!(" Found remote PingActor!");
                    break actor;
                }
                None => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        error!(" Failed to find PingActor after {} attempts", max_retries);
                        return Ok(());
                    }
                    warn!("⏳ PingActor not found yet, retrying... (attempt {}/{})", retry_count, max_retries);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        };

        info!(" Starting ping-pong sequence...");
        let start = Instant::now();

        // Send 100 ping messages with 2-second intervals
        for i in 1..=100 {
            let ping = Ping {
                message: format!("Hello from client, ping #{}", i),
                sequence: i,
            };

            info!(" Sending PING #{}", i);

            match remote_actor.ask(&ping).await {
                Ok(pong) => {
                    info!(" Received PONG #{} (total: {})", pong.sequence, pong.total_pings);
                }
                Err(e) => {
                    error!(" Failed: {}", e);
                }
            }

            if i < 100 {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }

        let duration = start.elapsed();
        info!(" Done! Total: {:?}, Avg: {:?}", duration, duration / 100);

        swarm_handle.abort();
        
    } else {
        info!(" No server address provided. Use --server flag.");
    }

    Ok(())
}