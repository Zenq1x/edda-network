use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, IdentTopic, MessageAuthenticity, ValidationMode},
    identify,
    kad::{self, store::MemoryStore},
    mdns,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, SwarmBuilder,
};
use tokio::sync::mpsc;

// ── Topics ────────────────────────────────────────────────────────────────────

pub const TOPIC_TX: &str    = "edda/tx/1";
pub const TOPIC_BLOCK: &str = "edda/block/1";

// ── Messages between network layer and the rest of the node ──────────────────

#[derive(Debug, Clone)]
pub enum InboundMessage {
    Transaction(Vec<u8>),
    Block(Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum OutboundMessage {
    BroadcastTransaction(Vec<u8>),
    BroadcastBlock(Vec<u8>),
}

// ── Combined network behaviour ────────────────────────────────────────────────

#[derive(NetworkBehaviour)]
pub struct EddaBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia:  kad::Behaviour<MemoryStore>,
    pub mdns:      mdns::tokio::Behaviour,
    pub identify:  identify::Behaviour,
}

// ── Main network struct ───────────────────────────────────────────────────────

pub struct EddaNetwork {
    swarm:       libp2p::Swarm<EddaBehaviour>,
    topic_tx:    IdentTopic,
    topic_block: IdentTopic,
    inbound_tx:  mpsc::Sender<InboundMessage>,
    outbound_rx: mpsc::Receiver<OutboundMessage>,
}

impl EddaNetwork {
    /// Create a new P2P node listening on `port`, optionally dialling `initial_peers`
    /// (multiaddr strings like `/ip4/1.2.3.4/tcp/7000`).
    pub async fn new(port: u16, initial_peers: Vec<String>) -> Result<(
        Self,
        mpsc::Receiver<InboundMessage>,
        mpsc::Sender<OutboundMessage>,
    )> {
        let topic_tx    = IdentTopic::new(TOPIC_TX);
        let topic_block = IdentTopic::new(TOPIC_BLOCK);

        let topic_tx_sub    = topic_tx.clone();
        let topic_block_sub = topic_block.clone();

        let mut swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let local_peer_id = PeerId::from_public_key(&key.public());

                // Gossipsub — for broadcasting txs and blocks
                let gs_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(1))
                    .validation_mode(ValidationMode::Strict)
                    .build()
                    .expect("valid gossipsub config");

                let mut gossipsub = gossipsub::Behaviour::new(
                    MessageAuthenticity::Signed(key.clone()),
                    gs_config,
                )
                .map_err(|e| anyhow::anyhow!(e))?;

                gossipsub.subscribe(&topic_tx_sub).unwrap();
                gossipsub.subscribe(&topic_block_sub).unwrap();

                // Kademlia DHT — for long-range peer discovery
                let kademlia = kad::Behaviour::new(
                    local_peer_id,
                    MemoryStore::new(local_peer_id),
                );

                // mDNS — instant discovery on local network (perfect for testnet)
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    local_peer_id,
                )
                .map_err(|e| anyhow::anyhow!(e))?;

                // Identify — nodes exchange protocol version and addresses
                let identify = identify::Behaviour::new(identify::Config::new(
                    "/edda/1.0.0".to_string(),
                    key.public(),
                ));

                Ok(EddaBehaviour { gossipsub, kademlia, mdns, identify })
            })?
            .build();

        // Start listening
        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", port).parse()?;
        swarm.listen_on(listen_addr)?;

        // Dial any bootstrap / seed peers supplied by the caller
        for addr_str in &initial_peers {
            match addr_str.parse::<Multiaddr>() {
                Ok(addr) => { swarm.dial(addr).ok(); }
                Err(e)   => println!("[P2P] Bad peer addr '{}': {}", addr_str, e),
            }
        }

        let (inbound_tx,  inbound_rx)  = mpsc::channel(256);
        let (outbound_tx, outbound_rx) = mpsc::channel(256);

        let network = Self {
            swarm,
            topic_tx,
            topic_block,
            inbound_tx,
            outbound_rx,
        };

        Ok((network, inbound_rx, outbound_tx))
    }

    /// Run the event loop — call this in a dedicated tokio task.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                // Handle outbound requests from the rest of the node
                Some(msg) = self.outbound_rx.recv() => {
                    match msg {
                        OutboundMessage::BroadcastTransaction(data) => {
                            match self.swarm.behaviour_mut().gossipsub
                                .publish(self.topic_tx.clone(), data)
                            {
                                Ok(_)  => println!("[P2P] Transaction broadcasted"),
                                Err(e) => println!("[P2P] Broadcast error: {:?}", e),
                            }
                        }
                        OutboundMessage::BroadcastBlock(data) => {
                            match self.swarm.behaviour_mut().gossipsub
                                .publish(self.topic_block.clone(), data)
                            {
                                Ok(_)  => println!("[P2P] Block broadcasted"),
                                Err(e) => println!("[P2P] Broadcast error: {:?}", e),
                            }
                        }
                    }
                }

                // Handle inbound swarm events
                event = self.swarm.next() => {
                    let Some(event) = event else { break };
                    self.handle_event(event).await;
                }
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<EddaBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("[P2P] Listening on {}", address);
            }

            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("[P2P] Connected   ↔ {}", peer_id);
            }

            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                println!("[P2P] Disconnected  {} ({:?})", peer_id, cause);
            }

            SwarmEvent::Behaviour(EddaBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                for (peer_id, addr) in peers {
                    println!("[P2P] mDNS found  {} @ {}", peer_id, addr);
                    self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                    // Try to connect automatically
                    let _ = self.swarm.dial(peer_id);
                }
            }

            SwarmEvent::Behaviour(EddaBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                for (peer_id, _) in peers {
                    println!("[P2P] mDNS expired {}", peer_id);
                }
            }

            SwarmEvent::Behaviour(EddaBehaviourEvent::Gossipsub(
                gossipsub::Event::Message { message, .. },
            )) => {
                let topic = message.topic.clone();
                let data  = message.data;

                if topic == self.topic_tx.hash() {
                    println!("[P2P] Received tx    ({} bytes)", data.len());
                    let _ = self.inbound_tx.send(InboundMessage::Transaction(data)).await;
                } else if topic == self.topic_block.hash() {
                    println!("[P2P] Received block ({} bytes)", data.len());
                    let _ = self.inbound_tx.send(InboundMessage::Block(data)).await;
                }
            }

            SwarmEvent::Behaviour(EddaBehaviourEvent::Gossipsub(
                gossipsub::Event::Subscribed { peer_id, topic },
            )) => {
                println!("[P2P] Peer {} subscribed to {}", peer_id, topic);
            }

            SwarmEvent::Behaviour(EddaBehaviourEvent::Identify(
                identify::Event::Received { peer_id, info, .. },
            )) => {
                println!("[P2P] Identified {} — {}", peer_id, info.protocol_version);
                for addr in info.listen_addrs {
                    self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                }
            }

            _ => {}
        }
    }
}
