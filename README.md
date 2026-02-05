# Distributed Ping-Pong with Kameo

A distributed actor system in Rust where two programs on different networks send ping-pong messages back and forth. Built with Kameo actors and libp2p.

## What is this?

This is a practical example of distributed computing. One program (the server) sits there waiting for pings. The other program (the client) connects directly and sends 100 ping messages, getting a response for each one.

The cool part? It works across different networks - even the internet. Successfully tested with the client on home WiFi connecting to a server on a university network via mobile hotspot. No VPN or port forwarding needed (as long as the server is reachable).

## Project Structure
```
kameo_ping/
├── ping-server/          # The listening/responding side
│   ├── src/main.rs
│   └── Cargo.toml
└── ping-client/          # The sending/requesting side
    ├── src/main.rs
    └── Cargo.toml
```

## Prerequisites

- Rust (install from https://rustup.rs)
- Two computers (or test on one machine)
- A bit of patience for the first compile (it downloads a lot of dependencies)

## Dependencies

Add these to both `Cargo.toml` files:

```
[dependencies]
kameo = { version = "0.19", features = ["remote"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
libp2p = "0.56"
clap = { version = "4", features = ["derive"] }
futures = "0.3"  
```

## Running It

### On Machine 1 (Server):
```bash
cd ping-server
cargo run --release
```

You'll see output like this:
```
INFO  Server Peer ID: 12D3KooWRTKTJBHZjgva43e73ETMaHk4GHU6vpQ9PdK6nxdV4Nh5
INFO  Listening on /ip4/127.0.0.1/tcp/36341
INFO  Connection address: /ip4/127.0.0.1/tcp/36341/p2p/12D3KooWRTKT...
INFO  Listening on /ip4/192.168.1.9/tcp/36341
INFO  Connection address: /ip4/192.168.1.9/tcp/36341/p2p/12D3KooWRTKT...
INFO  PingActor registered and ready!
```

**Copy the full connection address** - you need the entire thing including the peer ID. Use the one with your actual IP, not 127.0.0.1 (unless testing on the same machine).

### On Machine 2 (Client):

```bash
cd ping-client
cargo run --release -- --server "/ip4/192.168.1.9/tcp/36341/p2p/12D3KooWRTKT..."
```

Paste the **complete address** from the server output. Yes, the whole long string - that's how libp2p identifies peers.

The client will:
1. Connect to the server (1-2 seconds)
2. Wait for DHT to sync (15 seconds)
3. Look up the PingActor (a few retries, about 10-30 seconds)
4. Send 100 pings (with 2-second delays)

You'll see each ping and pong logged.

## Connection Examples

**Same machine (testing):**
```bash
--server "/ip4/127.0.0.1/tcp/36341/p2p/12D3KooW..."
```

**Same WiFi network:**
```bash
--server "/ip4/192.168.1.9/tcp/36341/p2p/12D3KooW..."
```

**Different networks (internet):**
```bash
--server "/ip4/128.2.220.206/tcp/36341/p2p/12D3KooW..."
```

## Verified Working Across Networks

This has been successfully tested with:
- **Client**: Laptop on home WiFi
- **Server**: Remote machine on mobile hotspot → university network

The DHT actor discovery works across different networks as long as the server is reachable. No special NAT traversal or port forwarding needed - just a direct connection.

## What happens under the hood

The programs use two networking layers:

1. **libp2p direct connection** - Establishes TCP with encryption (1-2 seconds)
2. **Kademlia DHT** - Finds the registered actor on the remote peer (15-30 seconds)

The delay is normal distributed systems behavior. The DHT needs time to propagate actor registration information between peers. It's like DNS propagation, but for actors.

## Network Setup

### Same Machine
Use `127.0.0.1` - works immediately, no configuration needed.

### Same WiFi/LAN
Both machines on the same local network. Allow incoming connections on port 36341 if your firewall asks.

### Different Networks (Internet)
This is the interesting case. It **just works** if:
- The server is on a machine with a reachable IP (university servers, cloud VMs, etc.)
- Client can establish TCP connection to server on port 36341

No port forwarding needed on the server side if it's already publicly accessible (like a university machine).

## Performance

With the default 2-second delay between pings, 100 round trips takes about 3-4 minutes.

Expected output:
```
INFO  Sending PING #1
INFO  Received PONG #1 (total: 1)
...
INFO  Sending PING #100
INFO  Received PONG #100 (total: 100)
INFO  Done! Total: 3m20s, Avg: 2s
```

The server keeps a total count of all pings received, so you can restart the client multiple times and watch the counter increment.

## Troubleshooting

### "PingActor not found yet, retrying..."

This is **normal** for the first several attempts. DHT takes 15-30 seconds to sync. If it keeps failing after a minute:

- Check that the server shows "Client connected" 
- Verify you copied the **complete** multiaddr including peer ID
- Make sure firewall allows port 36341

The warning "Failed to trigger bootstrap: No known peers" is **expected and harmless** - ignore it.

### Connection timeout or closed

- Firewall blocking port 36341
- Wrong IP address in the multiaddr
- Server not reachable from client network

**Quick checks:**
```bash
# Can you reach the server?
ping 128.2.220.206

# Test the port (Linux/Mac)
telnet 128.2.220.206 36341

# Test the port (Windows)
Test-NetConnection -ComputerName 128.2.220.206 -Port 36341
```
