use bytes::{BufMut, Bytes, BytesMut};
use futures::sync::mpsc;
use std::net::SocketAddr;
use tokio;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

struct Lines {
    socket: TcpStream,
    rd: BytesMut,
    wr: BytesMut,
}

impl Lines {
    fn new(socket: TcpStream) -> Self {
        Lines {
            socket,
            rd: BytesMut::new(),
            wr: BytesMut::new(),
        }
    }

    /// Can be polled to fill the internal `rd` buffer with bytes from the socket.
    /// Resolves when the socket is closed.
    fn fill_read_buf(&mut self) -> Result<Async<()>, io::Error> {
        loop {
            // Ensures there are at least 1024 more bytes in the read buffer
            self.rd.reserve(1024);
            let n = try_ready!(self.socket.read_buf(&mut self.rd));
            if n == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }

    /// Adds a line of data to the write buffer.
    fn buffer(&mut self, line: &[u8]) {
        self.wr.put(line);
    }

    /// Resolves when the current write buffers has been fully written to the socket
    fn poll_flush(&mut self) -> Poll<(), io::Error> {
        while !self.wr.is_empty() {
            // `n` is the number of bytes written
            let n = try_ready!(self.socket.poll_write(&self.wr));

            // if `wr` is non-empty, a successful write can never be 0 bytes.
            assert!(n > 0);

            // discard the first `n` bytes of the buffer
            self.wr.split_to(n);
        }
        Ok(Async::Ready(()))
    }
}

impl Stream for Lines {
    type Item = BytesMut;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        // self.fill_read_buf will complete when the socket closes
        let sock_closed = self.fill_read_buf()?.is_ready();

        // messages are delimited by \r\n, and `pos` is the index at which the message ends
        let pos = self.rd.windows(2).position(|bytes| bytes == b"\r\n");

        // If we find a line, chop it off and yield it
        if let Some(pos) = pos {
            // self.rd is now [pos + 2, len)
            let mut line = self.rd.split_to(pos + 2);
            // drop \r\n
            line.split_off(pos);
            // return the line
            return Ok(Async::Ready(Some(line)));
        }

        if sock_closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

fn process(socket: TcpStream, tx: mpsc::UnboundedSender<(SocketAddr, Bytes)>) {
    let addr = socket.peer_addr().unwrap();
    let lines = Lines::new(socket);
    let connection = lines
        .for_each(move |line| {
            tx.unbounded_send((addr, Bytes::from(line)));
            Ok(())
        })
        .map_err(|_| ());
    tokio::spawn(connection);
}

pub fn server(
    tx: mpsc::UnboundedSender<(SocketAddr, Bytes)>,
) -> impl Future<Item = (), Error = ()> {
    let addr = "0.0.0.0:1337".parse().unwrap();
    TcpListener::bind(&addr)
        .unwrap()
        .incoming()
        .for_each(move |socket: TcpStream| {
            process(socket, tx.clone());
            Ok(())
        })
        .map_err(|e| {
            println!("Error occurred in server: {:?}", e);
            ()
        })
}
