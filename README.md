# Distributed Ping-Pong with Kameo

A simple distributed actor system built with Rust and the Kameo framework. Two programs running on different machines that send ping-pong messages back and forth.

## What is this?

This is a basic example of distributed computing using actors. One program (the server) sits there waiting for pings. The other program (the client) connects and sends 100 ping messages, getting a response for each one.

The cool part is that they find each other automatically on the network - no hardcoded IPs needed. But if automatic discovery doesn't work on your network, you can specify the server's address manually.

## Project Structure
```
kameo_ping/
├── kameo_server/          # The listening/responding side
│   ├── src/main.rs
│   └── Cargo.toml
└── kameo_client/          # The sending/requesting side
    ├── src/main.rs
    └── Cargo.toml
```

## Prerequisites

- Rust (obviously) - install from https://rustup.rs
- Two computers on the same WiFi network
- A bit of patience for the first compile (it downloads a lot of dependencies)

## Running It

### On Machine 1 (Server):
```bash
cd kameo_server
cargo run --release
```

You'll see it print its peer ID and start listening. Note the listening address - you might need it for manual connection:
```
ActorSwarm listening on /ip4/192.168.1.13/tcp/62672
```

Just leave it running.

### On Machine 2 (Client):

**Option 1: Automatic Discovery (Default)**
```bash
cd kameo_client
cargo run --release
```

The client will search for the server (takes about 30 seconds) and then start sending pings.

**Option 2: Manual Connection**

If automatic discovery doesn't work (some networks block mDNS):
```bash
cd kameo_client
cargo run --release -- --server-ip 192.168.1.13 --port 62672
```

Use the IP and port that the server printed when it started.

## Connection Options

### Automatic Discovery (Recommended)

Works on most home and lab networks. Uses mDNS for peer discovery and Kademlia DHT for actor lookup. Takes 15-30 seconds to establish connection.
```bash
cargo run --release
```

### Manual Connection

For networks where mDNS is blocked (corporate/university networks, VPNs, etc.):
```bash
cargo run --release -- --server-ip <SERVER_IP> --port <PORT>
```

Example:
```bash
cargo run --release -- --server-ip 192.168.1.13 --port 62672
```

To see all available options:
```bash
cargo run --release -- --help
```

### Wide-Area Network (WAN) Testing

The manual connection option also enables testing across different networks. However, this requires additional network configuration:

- **Port Forwarding:** Forward the server's port on the router to its local IP, then use the router's public IP from the client
- **VPN:** Connect both machines to the same VPN (Tailscale, ZeroTier, etc.) and use VPN IPs

Without one of these setups, NAT will block connections between different networks.

## What happens under the hood

The programs use two networking layers:

1. **mDNS** - discovers peers on the local network (happens fast, under a second)
2. **Kademlia DHT** - finds the registered actor (this is the slow part, 15-30 seconds)

This is normal distributed systems stuff. The delay is because the DHT needs to propagate information across the network.

## Network Setup

Both machines need to be on the same local network for automatic discovery. If your Mac shows a firewall popup asking about incoming connections, click Allow.

The programs listen on random ports and discover each other automatically. If automatic discovery fails, check:
- Both machines are on the same WiFi
- Firewall allows incoming connections
- Router doesn't block multicast traffic (needed for mDNS)

## Performance

With the default 2-second delay between pings, 100 round trips takes about 3-4 minutes. Remove the sleep if you want to see how fast it can actually go.

The server keeps a count of all pings it's received, so you can restart the client multiple times and watch the count go up.

Performance metrics are displayed at the end:
- Total time for all round trips
- Average round-trip time per ping
