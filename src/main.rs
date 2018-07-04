extern crate localchat;
extern crate tokio;

use localchat::dnssd;
use localchat::peer::track_peers;
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

fn main() {
    let state = Arc::new(Mutex::new(State::new()));
    tokio::run(track_peers().unwrap());
}
