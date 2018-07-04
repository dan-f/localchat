extern crate localchat;
extern crate tokio;

use localchat::dnssd;
use localchat::peer::{track_peers, Peer, PeerEvent};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::prelude::*;

use localchat::NetworkEvent;

#[derive(Debug)]
struct State {
    service_registration: Option<dnssd::Registration>,
    peers: HashSet<Peer>,
}

impl State {
    fn new() -> Self {
        State {
            service_registration: None,
            peers: HashSet::new(),
        }
    }

    fn save_registration(&mut self, registration: dnssd::Registration) {
        self.service_registration = Some(registration);
    }

    fn add_peer(&mut self, peer: Peer) -> bool {
        self.peers.insert(peer)
    }

    fn drop_peer(&mut self, peer: &Peer) -> bool {
        self.peers.remove(&peer)
    }
}

fn register_service_task(
    state: Arc<Mutex<State>>,
) -> Result<impl Future<Item = (), Error = ()>, dnssd::Error> {
    let f = dnssd::register_service()?
        .and_then(move |registration| {
            let mut guard = state.lock().unwrap();
            (*guard).save_registration(registration);
            Ok(())
        })
        .map_err(|err| {
            println!("Error occurred registering service: {:?}", err);
        });
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
    let program_task = register_service_task(Arc::clone(&state))
        .unwrap()
        .and_then(move |_| track_peers_task(Arc::clone(&state)).unwrap());
    tokio::run(program_task);
}
