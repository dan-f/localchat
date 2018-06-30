extern crate localchat;
extern crate tokio;

use localchat::dnssd;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Delay;

#[derive(Debug)]
struct State {
    peers: HashSet<dnssd::Service>,
}

impl State {
    fn new() -> Self {
        State {
            peers: HashSet::new(),
        }
    }
}

fn register_service() -> Result<impl Future<Item = (), Error = ()>, dnssd::Error> {
    let f = dnssd::register_service()?
        .map_err(|err: dnssd::Error| {
            println!("Oh no, an error! {:?}", err);
            ()
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

fn track_peers(
    state: Arc<Mutex<State>>,
) -> Result<impl Future<Item = (), Error = ()>, dnssd::Error> {
    Ok(dnssd::browse_services()?
        .for_each(move |browse_event| {
            let mut guard = state.lock().unwrap();
            match browse_event {
                dnssd::BrowseEvent::Joined(service) => {
                    (*guard).peers.insert(service);
                }
                dnssd::BrowseEvent::Dropped(service) => {
                    (*guard).peers.remove(&service);
                }
            };
            println!("Peers: {:?}", (*guard).peers);
            Ok(())
        })
        .map_err(|e| {
            println!("Uh oh! {:?}", e);
            ()
        }))
}

fn main() {
    let state = Arc::new(Mutex::new(State::new()));
    tokio::run(track_peers(state).unwrap());
}
