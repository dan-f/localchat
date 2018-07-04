use std::net::SocketAddr;
use tokio::prelude::*;

use dnssd;

#[derive(Debug)]
pub struct Peer {
    pub servicename: String,
    pub hostname: String,
    pub socket_addr: SocketAddr,
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

pub fn track_peers() -> Result<impl Future<Item = (), Error = ()>, dnssd::Error> {
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
