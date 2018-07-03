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

fn track_peers() -> Result<impl Future<Item = (), Error = ()>, dnssd::Error> {
    let future = dnssd::browse_services()?
        .map(|browse_event| {
            match browse_event {
                dnssd::BrowseEvent::Joined(ref service) => {
                    println!("Service joined: {:?}", service);
                }
                dnssd::BrowseEvent::Dropped(ref service) => {
                    println!("Service dropped: {:?}", service);
                }
            }
            browse_event
        })
        .filter_map(|browse_event| match browse_event {
            dnssd::BrowseEvent::Joined(service) => Some(service),
            dnssd::BrowseEvent::Dropped(_) => None,
        })
        .for_each(|service| {
            dnssd::resolve_service(&service)
                .unwrap()
                .map(|host| {
                    println!("Resolved host: {:?}", host);
                    host
                })
                .and_then(|host| {
                    dnssd::get_address(&host)
                        .unwrap()
                        .map(|addr| println!("Address is: {:?}", addr))
                })
        })
        .map_err(|e| {
            println!("Uh oh! {:?}", e);
            ()
        });

    Ok(future)
}

fn main() {
    let state = Arc::new(Mutex::new(State::new()));
    tokio::run(track_peers().unwrap());
}
