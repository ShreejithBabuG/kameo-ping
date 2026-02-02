# Distributed Ping-Pong with Kameo

A simple distributed actor system built with Rust and the Kameo framework. Two programs running on different machines that send ping-pong messages back and forth.

## What is this?

This is a basic example of distributed computing using actors. One program (the server) sits there waiting for pings. The other program (the client) connects and sends 100 ping messages, getting a response for each one. The cool part is that they find each other automatically on the network, no hardcoded IPs needed.

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

You'll see it print its peer ID and start listening. Just leave it running.

### On Machine 2 (Client):
```bash
cd kameo_client
cargo run --release
```

The client will search for the server (takes about 30 seconds) and then start sending pings.

## What happens under the hood

The programs use two networking layers:

1. **mDNS** - discovers peers on the local network (happens fast, under a second)
2. **Kademlia DHT** - finds the registered actor (this is the slow part, 15-30 seconds)

This is normal distributed systems stuff. The delay is because the DHT needs to propagate information across the network.

## Network Setup

Both machines need to be on the same local network. If your Mac shows a firewall popup asking about incoming connections, click Allow. The programs listen on random ports and discover each other automatically.

## Performance

With the default 2-second delay between pings, 100 round trips takes about 3-4 minutes. Remove the sleep if you want to see how fast it can actually go. The server keeps a count of all pings it's received, so you can restart the client multiple times and watch the count go up.
