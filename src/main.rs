extern crate localchat;
extern crate tokio;

use localchat::dnssd;
use tokio::prelude::*;

fn main() -> Result<(), dnssd::Error> {
    let f = dnssd::register_service()?
        .then(|_| {
            println!("Service registered!");
            Ok(())
        })
        .map_err(|_: dnssd::Error| {
            println!("Oh no, an error!");
            ()
        });
    tokio::run(f);
    Ok(())
}
