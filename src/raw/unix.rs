mod c {
    extern crate libc;
    use std::os::raw::*;

    pub type SOCKET = c_int;
    pub const SOCKET_ERROR: c_int = -1;
    pub const SOCKET_SHUTDOWN: c_int = libc::ESHUTDOWN;

    //Structs
    pub use libc::{
        sockaddr,
        sockaddr_in,
        sockaddr_in6,
        sockaddr_storage
    };

    //Constants
    pub use libc::{
        AF_INET,
        AF_INET6,
        FIONBIO
    };

    //Functions
    pub use libc::{
        socket,
        getsockname,
        bind,
        listen,
        recv,
        recvfrom,
        send,
        sendto,
        accept,
        connect,
        getsockopt,
        setsockopt,
        ioctl,
        shutdown,
        close
    };
}

use self::c::*;

///Type of socket's shutdown operation.
#[derive(Copy, Clone)]
pub enum ShutdownType {
    ///Stops any further receives.
    Receive = 0,
    ///Stops any further sends.
    Send = 1,
    ///Stops both sends and receives.
    Both = 2
}

impl Into<c_int> for ShutdownType {
    fn into(self) -> c_int {
        self as c_int
    }
}

///Raw socket
pub struct Socket {
    inner: SOCKET
}

impl Socket {
    ///Initializes new socket.
    ///
    ///Corresponds to C connect()
    pub fn new(family: c_int, _type: c_int, protocol: c_int) -> io::Result<Socket> {
        unsafe {
            match socket(family, _type, protocol) {
                SOCKET_ERROR => Err(io::Error::last_os_error()),
                fd => Ok(Socket {
                    inner: fd
                }),
            }
        }
    }

    ///Returns underlying socket descriptor.
    ///
    ///Note: ownership is not transferred.
    pub fn raw(&self) -> SOCKET {
        self.inner
    }

    ///Retrieves socket name i.e. address
    ///
    ///Wraps `getsockname()`
    ///
    ///Available for binded/connected sockets.
    pub fn name(&self) -> io::Result<net::SocketAddr> {
        unsafe {
            let mut storage: sockaddr_storage = mem::zeroed();
            let mut len = mem::size_of_val(&storage) as c_int;

            match getsockname(self.inner, &mut storage as *mut _ as *mut _, &mut len) {
                SOCKET_ERROR => Err(io::Error::last_os_error()),
                _ => sockaddr_to_addr(&storage, len)
            }
        }
    }

    ///Binds socket to address.
    pub fn bind(&self, addr: &net::SocketAddr) -> io::Result<()> {
        let (addr, len) = get_raw_addr(addr);

        unsafe {
            match bind(self.inner, addr, len) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Listens for incoming connections on this socket.
    pub fn listen(&self, backlog: c_int) -> io::Result<()> {
        unsafe {
            match listen(self.inner, backlog) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Receives some bytes from socket
    ///
    ///Number of received bytes is returned on success
    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        let len = cmp::min(buf.len(), i32::max_value() as usize) as i32;
        unsafe {
            match recv(self.inner, buf.as_mut_ptr() as *mut c_char, len, 0) {
                SOCKET_ERROR => Err(io::Error::last_os_error()),
                n => Ok(n as usize)
            }
        }
    }

    ///Receives some bytes from socket
    ///
    ///Number of received bytes and remote address are returned on success.
    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, net::SocketAddr)> {
        let len = cmp::min(buf.len(), i32::max_value() as usize) as i32;
        unsafe {
            let mut storage: sockaddr_storage = mem::zeroed();
            let mut storage_len = mem::size_of_val(&storage) as c_int;

            match recvfrom(self.inner, buf.as_mut_ptr() as *mut c_char, len, 0, &mut storage as *mut _ as *mut _, &mut storage_len) {
                SOCKET_ERROR => Err(io::Error::last_os_error()),
                n => {
                    let peer_addr = sockaddr_to_addr(&storage, storage_len)?;
                    Ok((n as usize, peer_addr))
                }
            }
        }
    }

    ///Sends some bytes through socket.
    ///
    ///Number of sent bytes is returned.
    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
        let len = cmp::min(buf.len(), i32::max_value() as usize) as i32;

        unsafe {
            match send(self.inner, buf.as_ptr() as *const c_char, len, 0) {
                SOCKET_ERROR => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == SOCKET_SHUTDOWN {
                        Ok(0)
                    }
                    else {
                        Err(error)
                    }
                },
                n => Ok(n as usize)
            }
        }
    }

    ///Sends some bytes through socket toward specified peer.
    ///
    ///Number of sent bytes is returned.
    ///
    ///Note: the socket will be bound, if it isn't already.
    ///Use method `name` to determine address.
    pub fn send_to(&self, buf: &[u8], peer_addr: &net::SocketAddr) -> io::Result<usize> {
        let len = cmp::min(buf.len(), i32::max_value() as usize) as i32;
        let (addr, addr_len) = get_raw_addr(peer_addr);

        unsafe {
            match sendto(self.inner, buf.as_ptr() as *const c_char, len, 0, addr, addr_len) {
                SOCKET_ERROR => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == SOCKET_SHUTDOWN {
                        Ok(0)
                    }
                    else {
                        Err(error)
                    }
                },
                n => Ok(n as usize)
            }
        }
    }

    ///Accepts incoming connection.
    pub fn accept(&self) -> io::Result<(Socket, net::SocketAddr)> {
        unsafe {
            let mut storage: sockaddr_storage = mem::zeroed();
            let mut len = mem::size_of_val(&storage) as c_int;

            match accept(self.inner, &mut storage as *mut _ as *mut _, &mut len) {
                SOCKET_ERROR => Err(io::Error::last_os_error()),
                sock @ _ => {
                    let addr = sockaddr_to_addr(&storage, len)?;
                    Ok((Socket { inner: sock, }, addr))
                }
            }
        }
    }

    ///Connects socket with remote address.
    pub fn connect(&self, addr: &net::SocketAddr) -> io::Result<()> {
        let (addr, len) = get_raw_addr(addr);

        unsafe {
            match connect(self.inner, addr, len) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Retrieves socket option.
    pub fn get_opt<T>(&self, level: c_int, name: c_int) -> io::Result<T> {
        unsafe {
            let mut value: T = mem::zeroed();
            let value_ptr = &mut value as *mut T as *mut c_char;
            let mut value_len = mem::size_of::<T>() as c_int;

            match getsockopt(self.inner, level, name, value_ptr, &mut value_len) {
                0 => Ok(value),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Sets socket option
    ///
    ///Value is generally integer or C struct.
    pub fn set_opt<T>(&self, level: c_int, name: c_int, value: T) -> io::Result<()> {
        unsafe {
            let value = &value as *const T as *const c_char;

            match setsockopt(self.inner, level, name, value, mem::size_of::<T>() as c_int) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Sets I/O parameters of socket.
    ///
    ///It uses `ioctlsocket` under hood.
    pub fn ioctl(&self, request: c_int, value: c_ulong) -> io::Result<()> {
        unsafe {
            let mut value = value;
            let value = &mut value as *mut c_ulong;

            match ioctl(self.inner, request, value) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Sets non-blocking mode.
    pub fn set_nonblocking(&self, value: bool) -> io::Result<()> {
        self.ioctl(FIONBIO as c_long, value as c_ulong)
    }


    ///Stops receive and/or send over socket.
    pub fn shutdown(&self, direction: ShutdownType) -> io::Result<()> {
        unsafe {
            match shutdown(self.inner, direction.into()) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Closes socket.
    ///
    ///Note: on `Drop` socket will be closed on its own.
    ///There is no need to close it explicitly.
    pub fn close(&self) -> io::Result<()> {
        unsafe {
            match close(self.inner) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }
}

fn get_raw_addr(addr: &net::SocketAddr) -> (*const sockaddr, c_int) {
    match *addr {
        net::SocketAddr::V4(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as c_int)
        }
        net::SocketAddr::V6(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as c_int)
        }
    }
}

fn sockaddr_to_addr(storage: &sockaddr_storage, len: c_int) -> io::Result<net::SocketAddr> {
    match storage.ss_family as c_int {
        AF_INET => {
            assert!(len as usize >= mem::size_of::<sockaddr_in>());
            let storage = unsafe { *(storage as *const _ as *const sockaddr_in) };
            let ip = net::Ipv4Addr::new(storage.sin_addr.s_addr[0],
                                        storage.sin_addr.s_addr[1],
                                        storage.sin_addr.s_addr[2],
                                        storage.sin_addr.s_addr[3]);

            //Note to_be() swap bytes on LE targets
            //As IP stuff is always BE, we need swap only on LE targets
            Ok(net::SocketAddr::V4(net::SocketAddrV4::new(ip, storage.sin_port.to_be())))
        }
        AF_INET6 => {
            assert!(len as usize >= mem::size_of::<sockaddr_in6>());
            let storage = unsafe { *(storage as *const _ as *const sockaddr_in6) };
            let ip = net::Ipv6Addr::new(storage.sin6_addr.s6_addr[0],
                                        storage.sin6_addr.s6_addr[1],
                                        storage.sin6_addr.s6_addr[2],
                                        storage.sin6_addr.s6_addr[3],
                                        storage.sin6_addr.s6_addr[4],
                                        storage.sin6_addr.s6_addr[5],
                                        storage.sin6_addr.s6_addr[6],
                                        storage.sin6_addr.s6_addr[7]);

            Ok(net::SocketAddr::V6(net::SocketAddrV6::new(ip, storage.sin6_port.to_be(), storage.sin6_flowinfo, storage.sin6_scope_id)))
        }
        _ => {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid addr type."))
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        let _ = self.shutdown(ShutdownType::Both);
        let _ = self.close();
    }
}
