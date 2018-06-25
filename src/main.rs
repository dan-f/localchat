extern crate localchat;
extern crate tokio;

use localchat::dnssd;
use tokio::prelude::*;

fn main() -> Result<(), dnssd::Error> {
    let f = dnssd::register_service()?
        .then(|service| {
            println!("Service registered: {:?}", service);
            Ok(())
        })
        .map_err(|_: dnssd::Error| {
            println!("Oh no, an error!");
            ()
        });
    tokio::run(f);
    Ok(())
}
