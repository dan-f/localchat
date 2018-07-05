use futures::sync::mpsc;
use std::net::SocketAddr;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

// fn process(socket: TcpStream, tx: mpsc::UnboundedSender<(SocketAddr, String)>) {
//     let addr = socket.peer_addr().unwrap();
//     tx.unbounded_send((addr, String::from("Hello!")));
// }

pub fn server(
    tx: mpsc::UnboundedSender<(SocketAddr, String)>,
) -> impl Future<Item = (), Error = ()> {
    let addr = "0.0.0.0:1337".parse().unwrap();
    TcpListener::bind(&addr)
        .unwrap()
        .incoming()
        .for_each(move |socket: TcpStream| {
            let tx = tx.clone();
            tx.unbounded_send((addr, String::from("Hello!")));
            // process(socket, tx);
            Ok(())
        })
        .map_err(|e| {
            println!("Error occurred in server: {:?}", e);
            ()
        })
}
