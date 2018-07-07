#[macro_use]
extern crate futures;
extern crate bytes;
extern crate libc;
extern crate mio;
extern crate tokio;

pub mod chat;
pub mod dnssd;
pub mod peer;

#[derive(Clone, Debug)]
pub enum NetworkEvent {
    Joined,
    Dropped,
}
