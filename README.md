# lazy-socket

[![Build status](https://ci.appveyor.com/api/projects/status/gmh944j6an9btfka/branch/master?svg=true)](https://ci.appveyor.com/project/DoumanAsh/lazy-socket-rs/branch/master)
[![Build Status](https://travis-ci.org/DoumanAsh/lazy-socket.rs.svg?branch=master)](https://travis-ci.org/DoumanAsh/lazy-socket.rs)
[![Crates.io](https://img.shields.io/crates/v/lazy-socket.svg)](https://crates.io/crates/lazy-socket)
[![Docs.rs](https://docs.rs/lazy-socket/badge.svg)](https://docs.rs/crate/lazy-socket/)

The library serves as thin wrapper over socket API.
It provides necessary minimum amount of safety and easy to use.

## Obsolete kinda

It seems I wouldn't need this library anymore as there is [socket2](https://github.com/alexcrichton/socket2-rs)
by [alexcrichton](https://github.com/alexcrichton).

He is definitely more trustworthy and my library might as well be forgotten.
So go socket2 instead.

## Examples

### Create TCP socket and connect in non-blocking mode.

```rust
extern crate lazy_socket;

use std::net;
use std::str::FromStr;

use lazy_socket::raw::{
    Socket,
    Family,
    Protocol,
    Type,
    select
};

fn main() {
    let timeout = 1000;
    let socket = match Socket::new(Family::IPv4, Type::STREAM, Protocol::TCP) {
        Ok(socket) => socket,
        Err(error) => {
            println!("Couldn't open socket. Erro: {}", error);
            return;
        }
    };

    let dest = net::SocketAddr::from_str("192.168.0.1:80").unwrap();

    let _ = socket.set_blocking(false);
    let _ = socket.connect(&dest);
    match select(&[], &[&socket], &[&socket], Some(timeout)) {
          Ok(_) => println!("Connected!"),
          Err(error) => println!("Failed to connect. Error:{}", error)
    }
}
```
