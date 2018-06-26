extern crate localchat;
extern crate tokio;

use localchat::dnssd;
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Delay;

fn main() -> Result<(), dnssd::Error> {
    let f = dnssd::register_service()?
        .map_err(|err: dnssd::Error| {
            println!("Oh no, an error! {:?}", err);
            ()
        })
        .then(|registration| {
            let dur = Duration::from_secs(5);
            println!("Registered! Will deregister at {:?}", dur);
            Delay::new(Instant::now() + dur).map(move |_| {
                println!("Registration: {:?}", registration);
                // Deregistration happens when `registration` drops
            })
        })
        .map_err(|_| ());
    tokio::run(f);
    Ok(())
}
