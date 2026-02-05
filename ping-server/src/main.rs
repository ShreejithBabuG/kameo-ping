use kameo::prelude::*;
use kameo::remote;
use libp2p::{
    noise, tcp, yamux,
    swarm::{NetworkBehaviour, SwarmEvent, dial_opts::DialOpts},
    Multiaddr,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;
use futures::StreamExt;

// Custom network behaviour wrapping Kameo's remote behaviour
// Required for custom swarm configuration
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    kameo: remote::Behaviour,
}

// PingActor maintains a count of received pings
#[derive(Actor)]
pub struct PingActor {
    ping_count: u64,
}

// REMOTE_ID must match between client and server for discovery
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

// UUID must match between client and server for proper message routing
#[remote_message("a1b2c3d4-e5f6-7890-abcd-ef1234567890")]
impl Message<Ping> for PingActor {
    type Reply = Pong;

    async fn handle(&mut self, msg: Ping, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.ping_count += 1;
        info!(" Received PING #{} with message: '{}'", msg.sequence, msg.message);

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
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    info!(" Starting Ping Server with custom swarm...");

    // Build custom libp2p swarm with TCP, noise encryption, and yamux multiplexing
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

    // Initialize Kameo's global actor registry
    swarm.behaviour().kameo.init_global();

    let peer_id = *swarm.local_peer_id();
    info!(" Server Peer ID: {}", peer_id);

    // Listen on all interfaces on port 36341
    swarm.listen_on("/ip4/0.0.0.0/tcp/36341".parse()?)?;

    // Spawn and register the PingActor in background task to avoid blocking swarm
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let ping_actor = PingActor::spawn(PingActor { ping_count: 0 });
        match ping_actor.register("ping_actor").await {
            Ok(_) => info!(" PingActor registered and ready!"),
            Err(e) => info!(" Failed to register actor: {}", e),
        }
    });

    info!("⏳ Waiting for connections...");

    // Main event loop processing swarm events
    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::Behaviour(MyBehaviourEvent::Kameo(event)) => {
                        info!(" Kameo event: {:?}", event);
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        info!(" Client connected: {} via {}", peer_id, endpoint.get_remote_address());
                        
                        // Add peer to address book for Kademlia DHT discovery
                        let remote_addr = endpoint.get_remote_address().clone();
                        swarm.add_peer_address(peer_id, remote_addr.clone());
                        info!(" Added peer address to swarm: {} at {}", peer_id, remote_addr);
                    }
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        info!(" Client disconnected: {} ({:?})", peer_id, cause);
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(" Listening on {}", address);
                        let addr_string = address.to_string();
                        let addr_parts: Vec<&str> = addr_string.split('/').collect();
                        if addr_parts.len() >= 3 {
                            info!(" Connection address: /ip4/{}/tcp/36341/p2p/{}", addr_parts[2], peer_id);
                        }
                    }
                    SwarmEvent::IncomingConnection { .. } => {
                        info!(" Incoming connection...");
                    }
                    _ => {}
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!(" Shutting down server...");
                break;
            }
        }
    }

    Ok(())
}