extern crate lazy_socket;

use std::net;
use std::str::FromStr;
use std::os::raw::*;
use lazy_socket::raw::Socket;

#[test]
fn socket_new_raw_icmp() {
    //Test requires admin priviligies.
    let family: c_int = 2;
    let ty: c_int = 3;
    let proto: c_int = 1;
    let addr = net::SocketAddr::from_str("0.0.0.0:0").unwrap();

    let socket = Socket::new(family, ty, proto);
    assert!(socket.is_ok());
    let socket = socket.unwrap();

    //Before bind it shouldn't be possible to get name.
    let socket_name = socket.name();
    assert!(socket_name.is_err());

    assert!(socket.bind(&addr).is_ok());

    let socket_name = socket.name();
    assert!(socket_name.is_ok());
    let socket_name = socket_name.unwrap();

    assert_eq!(socket_name, addr);
}
