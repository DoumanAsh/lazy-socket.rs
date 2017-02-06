extern crate lazy_socket;
#[cfg(unix)]
extern crate libc;

use std::thread;
use std::net;
use std::str::FromStr;
use std::os::raw::*;
use lazy_socket::raw::Socket;
use std::time;

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
    let level: c_int = libc::SOL_SOCKET; //SOL_SOCKET
    #[cfg(windows)]
    let name: c_int = 0x0004; //SO_REUSEADDR
    #[cfg(unix)]
    let name: c_int = libc::SO_REUSEADDR; //SO_REUSEADDR

    let socket = Socket::new(family, ty, proto).unwrap();

    let result = socket.get_opt::<c_int>(level, name);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);

    let result = socket.set_opt(level, name, value_true);
    assert!(result.is_ok());

    let result = socket.get_opt::<c_int>(level, name);
    assert!(result.is_ok());

    //TODO: For some reason OSX returns here 4
    //      Need to find out what is wrong
    //      For now as long as it non zero it is true.
    #[cfg(target_os = "macos")]
    assert!(result.unwrap() > 0);
    #[cfg(not(target_os = "macos"))]
    assert_eq!(result.unwrap(), value_true);

    assert!(socket.set_nonblocking(true).is_ok());
    assert!(socket.set_nonblocking(false).is_ok());
}

#[cfg(windows)]
#[test]
fn socket_as_into_from_traits() {
    use std::os::windows::io::{
        AsRawSocket,
        FromRawSocket,
        IntoRawSocket,
    };

    let family: c_int = 2;
    let ty: c_int = 1;
    let proto: c_int = 6;

    let raw_socket;

    {
        let socket = Socket::new(family, ty, proto).unwrap();
        raw_socket = socket.into_raw_socket();
    }

    let socket = unsafe { Socket::from_raw_socket(raw_socket) };

    assert_eq!(raw_socket, socket.as_raw_socket());
    assert!(socket.close().is_ok());
}

#[cfg(unix)]
#[test]
fn socket_as_into_from_traits() {
    use std::os::unix::io::{
        AsRawFd,
        FromRawFd,
        IntoRawFd,
    };

    let family: c_int = 2;
    let ty: c_int = 1;
    let proto: c_int = 6;

    let raw_socket;

    {
        let socket = Socket::new(family, ty, proto).unwrap();
        raw_socket = socket.into_raw_fd();
    }

    let socket = unsafe { Socket::from_raw_fd(raw_socket) };

    assert_eq!(raw_socket, socket.as_raw_fd());
    assert!(socket.close().is_ok());
}

#[test]
fn socket_select_timeout() {
    let timeout = 100;
    #[cfg(windows)]
    let would_block_errno = 10035;
    #[cfg(unix)]
    let would_block_errno = libc::EINPROGRESS;

    let family: c_int = 2;
    let ty: c_int = 1;
    let proto: c_int = 6;
    let server_addr = net::SocketAddr::from_str("222.0.0.1:60004").unwrap();

    let client = Socket::new(family, ty, proto).unwrap();

    assert!(client.set_nonblocking(true).is_ok());
    let result = client.connect(&server_addr);
    assert!(result.is_err()); //Non-blocking connect returns error
    assert_eq!(result.err().unwrap().raw_os_error().unwrap(), would_block_errno);

    let now = time::Instant::now();
    let result = lazy_socket::raw::select(&[], &[&client], &[&client], Some(timeout));
    let elapsed = now.elapsed();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
    let elapsed = elapsed.as_secs() / 1000 + elapsed.subsec_nanos() as u64 / 1000000;
    assert!(elapsed >= (timeout - 50) || elapsed <= (timeout + 50));
}

#[test]
fn socket_select_connect() {
    #[cfg(windows)]
    let would_block_errno = 10035;
    #[cfg(unix)]
    let would_block_errno = libc::EINPROGRESS;

    let family: c_int = 2;
    let ty: c_int = 1;
    let proto: c_int = 6;
    let server_addr = net::SocketAddr::from_str("127.0.0.1:60006").unwrap();

    let server = Socket::new(family, ty, proto).unwrap();
    assert!(server.bind(&server_addr).is_ok());
    assert!(server.listen(1).is_ok());

    let client = Socket::new(family, ty, proto).unwrap();

    let th = thread::spawn(move || {
        let result = server.accept();
        assert!(result.is_ok());
    });

    assert!(client.set_nonblocking(true).is_ok());
    let result = client.connect(&server_addr);
    assert!(result.is_err()); //Non-blocking connect returns error
    println!("result={:?}", result);
    assert_eq!(result.err().unwrap().raw_os_error().unwrap(), would_block_errno);

    let result = lazy_socket::raw::select(&[], &[&client], &[&client], None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    assert!(th.join().is_ok());
}
