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

#[test]
fn socket_test_udp() {
    let family: c_int = 2;
    let ty: c_int = 2;
    let proto: c_int = 17;
    let data = [1, 2, 3, 4];
    let addr = net::SocketAddr::from_str("127.0.0.1:666").unwrap();

    let server = Socket::new(family, ty, proto).unwrap();
    assert!(server.bind(&addr).is_ok());
    let server_addr = server.name().unwrap();
    assert_eq!(addr, server_addr);

    let client = Socket::new(family, ty, proto).unwrap();
    assert!(client.bind(&net::SocketAddr::from_str("127.0.0.1:5666").unwrap()).is_ok());
    let client_addr = client.name().unwrap();

    let result = client.send_to(&data, &addr);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result, data.len());

    let mut read_data = [0; 10];

    // recv_from
    let result = server.recv_from(&mut read_data);
    assert!(result.is_ok());
    let (result_len, result_addr) = result.unwrap();

    assert_eq!(result_len, data.len());
    assert_eq!(read_data[result_len], 0);
    assert_eq!(result_addr, client_addr);
    assert_eq!(&read_data[..result_len], data);

    // 2 send + 2 recv
    let result = client.send_to(&data, &addr);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result, data.len());

    let result = client.send_to(&data, &addr);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result, data.len());

    let result = server.recv(&mut read_data);
    assert!(result.is_ok());
    let result_len = result.unwrap();
    assert_eq!(result_len, data.len());
    assert_eq!(read_data[result_len], 0);
    assert_eq!(&read_data[..result_len], data);

    let result = server.recv(&mut read_data);
    assert!(result.is_ok());
    let result_len = result.unwrap();
    assert_eq!(result_len, data.len());
    assert_eq!(read_data[result_len], 0);
    assert_eq!(&read_data[..result_len], data);
}
