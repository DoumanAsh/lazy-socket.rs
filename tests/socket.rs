extern crate lazy_socket;

use std::thread;
use std::net;
use std::str::FromStr;
use std::os::raw::*;
use lazy_socket::raw::Socket;

#[test]
fn socket_new_raw_icmp() {
    //Test requires admin privileges.
    let family: c_int = 2;
    let ty: c_int = 3;
    let proto: c_int = 1;
    let addr = net::SocketAddr::from_str("0.0.0.0:0").unwrap();

    let socket = Socket::new(family, ty, proto);

    if let Err(error) = socket {
        let error_code = error.raw_os_error().unwrap();

        #[cfg(windows)]
        assert_eq!(error_code, 10013); //Error code for insufficient admin rights.
        #[cfg(unix)]
        assert_eq!(error_code, 1);
        //We can skip in this case.
        return;
    }

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
    let addr = net::SocketAddr::from_str("127.0.0.1:1666").unwrap();

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

#[test]
fn socket_test_tcp() {
    let family: c_int = 2;
    let ty: c_int = 1;
    let proto: c_int = 6;
    let data = [1, 2, 3, 4];
    let server_addr = net::SocketAddr::from_str("127.0.0.1:60000").unwrap();
    let client_addr = net::SocketAddr::from_str("127.0.0.1:65003").unwrap();

    let server = Socket::new(family, ty, proto).unwrap();
    assert!(server.bind(&server_addr).is_ok());
    let addr = server.name().unwrap();
    assert_eq!(addr, server_addr);
    assert!(server.listen(1).is_ok());

    let client = Socket::new(family, ty, proto).unwrap();
    assert!(client.bind(&client_addr).is_ok());
    let addr = client.name().unwrap();
    assert_eq!(addr, client_addr);

    let th = thread::spawn(move || {
        let result = server.accept();
        assert!(result.is_ok());
        let (result_socket, result_addr) = result.unwrap();

        assert_eq!(result_addr, client_addr);

        let mut buf = [0; 10];
        let result = result_socket.recv(&mut buf);
        assert!(result.is_ok());
        let result_len = result.unwrap();
        assert_eq!(result_len, data.len());
        assert_eq!(buf[result_len], 0);
        assert_eq!(&buf[..result_len], data);
    });

    let result = client.connect(&server_addr);
    assert!(result.is_ok());
    assert!(client.send(&data).is_ok());

    assert!(th.join().is_ok());
}

#[test]
fn socket_test_options() {
    let value_true: c_int = 1;
    let family: c_int = 2;
    let ty: c_int = 1;
    let proto: c_int = 6;

    #[cfg(windows)]
    let level: c_int = 0xffff; //SOL_SOCKET
    #[cfg(unix)]
    let level: c_int = 1; //SOL_SOCKET
    #[cfg(windows)]
    let name: c_int = 0x0004; //SO_REUSEADDR
    #[cfg(unix)]
    let name: c_int = 2; //SO_REUSEADDR

    let socket = Socket::new(family, ty, proto).unwrap();

    let result = socket.get_opt::<bool>(level, name);
    assert!(result.is_ok());
    assert!(!result.unwrap());

    let result = socket.set_opt(level, name, value_true);
    assert!(result.is_ok());

    let result = socket.get_opt::<bool>(level, name);
    assert!(result.is_ok());
    assert!(result.unwrap());

    assert!(socket.set_nonblocking(true).is_ok());
    assert!(socket.set_nonblocking(false).is_ok());
}
