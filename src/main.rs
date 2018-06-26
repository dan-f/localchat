extern crate localchat;
extern crate tokio;

use localchat::dnssd;
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Delay;

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

fn browse_services() -> Result<impl Future<Item = (), Error = ()>, dnssd::Error> {
    Ok(dnssd::browse_services()?
        .for_each(|service| {
            println!("Found service: {:?}", service);
            Ok(())
        })
        .map_err(|e| {
            println!("Uh oh! {:?}", e);
            ()
        }))
}

fn main() {
    tokio::run(browse_services().unwrap());
}
