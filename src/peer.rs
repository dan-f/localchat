use std::net::SocketAddr;
use tokio::prelude::*;

use super::NetworkEvent;

use dnssd;

#[derive(Debug)]
pub struct PeerWithEvent {
    peer: Peer,
    event: NetworkEvent,
}

#[derive(Debug)]
pub struct Peer {
    pub servicename: String,
    pub hostname: String,
    pub socket_addr: SocketAddr,
}

fn find_peer(service: &dnssd::Service) -> impl Future<Item = Peer, Error = dnssd::Error> {
    let servicename = service.name.clone();
    // TODO: remove these unwraps
    dnssd::resolve_service(&service).unwrap().and_then(|host| {
        dnssd::get_address(&host).unwrap().map(|addr| Peer {
            servicename: servicename,
            hostname: host.name,
            socket_addr: SocketAddr::new(addr, host.port),
        })
    })
}

pub fn track_peers() -> Result<impl Stream<Item = PeerWithEvent, Error = dnssd::Error>, dnssd::Error>
{
    Ok(
        dnssd::browse_services()?.and_then(|dnssd::BrowseEvent { service, event }| {
            find_peer(&service).map(|peer| PeerWithEvent { peer, event })
        }),
    )
}
