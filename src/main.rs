extern crate localchat;
extern crate tokio;

use localchat::dnssd;
use std::collections::HashSet;
use std::net::SocketAddr;
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

#[derive(Debug)]
struct Peer {
    servicename: String,
    hostname: String,
    socket_addr: SocketAddr,
}

fn find_peer(service: &dnssd::Service) -> impl Future<Item = Peer, Error = dnssd::Error> {
    let servicename = service.name.clone();
    dnssd::resolve_service(&service).unwrap().and_then(|host| {
        dnssd::get_address(&host).unwrap().map(|addr| Peer {
            servicename: servicename,
            hostname: host.name,
            socket_addr: SocketAddr::new(addr, host.port),
        })
    })
}

fn track_peers() -> Result<impl Future<Item = (), Error = ()>, dnssd::Error> {
    let future = dnssd::browse_services()?
        .for_each(|event| {
            let print_peer = |service| find_peer(&service).map(|peer| println!("{:?}", peer));
            match event {
                dnssd::BrowseEvent::Joined(service) => {
                    print!("Peer joined: ");
                    print_peer(service)
                }
                dnssd::BrowseEvent::Dropped(service) => {
                    print!("Peer dropped: ");
                    print_peer(service)
                }
            }
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
