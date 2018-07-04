extern crate localchat;
extern crate tokio;

use localchat::dnssd;
use localchat::peer::{track_peers, Peer, PeerEvent};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Delay;

use localchat::NetworkEvent;

#[derive(Debug)]
struct State {
    peers: HashSet<Peer>,
}

impl State {
    fn new() -> Self {
        State {
            peers: HashSet::new(),
        }
    }

    fn add_peer(&mut self, peer: Peer) -> bool {
        self.peers.insert(peer)
    }

    fn drop_peer(&mut self, peer: &Peer) -> bool {
        self.peers.remove(&peer)
    }
}

fn register_service() -> Result<impl Future<Item = (), Error = ()>, dnssd::Error> {
    let f = dnssd::register_service()?
        .map_err(|err| {
            println!("Oh no, an error! {:?}", err);
        })
        .then(|registration| {
            let dur = Duration::from_secs(5);
            println!("Registered! Will deregister at {:?}", dur);
            Delay::new(Instant::now() + dur).map(move |_| {
                println!("Registered and deregistered: {:?}", registration);
                // Deregistration happens when `registration` drops
            })
        })
        .map_err(|_| ());
    Ok(f)
}

fn track_peers_task(
    state: Arc<Mutex<State>>,
) -> Result<impl Future<Item = (), Error = ()>, dnssd::Error> {
    let task = track_peers()?
        .for_each(move |peer_event| {
            let PeerEvent { peer, event } = peer_event;
            match event {
                NetworkEvent::Joined => {
                    let mut guard = state.lock().unwrap();
                    (*guard).add_peer(peer);
                }
                NetworkEvent::Dropped => {
                    let mut guard = state.lock().unwrap();
                    (*guard).drop_peer(&peer);
                }
            }
            println!("State: {:?}", state);
            Ok(())
        })
        .map_err(|err| {
            println!("Error occurred tracking peers: {:?}", err);
            ()
        });
    Ok(task)
}

fn main() {
    let state = Arc::new(Mutex::new(State::new()));
    tokio::run(track_peers_task(Arc::clone(&state)).unwrap());
}
