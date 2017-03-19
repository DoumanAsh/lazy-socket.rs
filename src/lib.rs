//!Raw Socket API
//!
//!The library serves as thin wrapper over socket API.
//!It provides necessary minimum amount of safety and easy to use.
//!
//!## Examples
//!
//!### Create TCP socket and connect in non-blocking mode.
//!
//!```rust
//!extern crate lazy_socket;
//!
//!use std::net;
//!use std::str::FromStr;
//!
//!use lazy_socket::raw::{
//!    Socket,
//!    Family,
//!    Protocol,
//!    Type,
//!    select
//!};
//!
//!fn main() {
//!    let timeout = 1000;
//!    let socket = match Socket::new(Family::IPv4, Type::STREAM, Protocol::TCP) {
//!        Ok(socket) => socket,
//!        Err(error) => {
//!            println!("Couldn't open socket. Erro: {}", error);
//!            return;
//!        }
//!    };
//!
//!    let dest = net::SocketAddr::from_str("192.168.0.1:80").unwrap();
//!
//!    let _ = socket.set_blocking(false);
//!    let _ = socket.connect(&dest);
//!    match select(&[], &[&socket], &[&socket], Some(timeout)) {
//!          Ok(_) => println!("Connected!"),
//!          Err(error) => println!("Failed to connect. Error:{}", error)
//!    }
//!}
//!```

#[macro_use]
extern crate bitflags;

pub mod raw;
