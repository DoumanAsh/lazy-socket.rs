use std::io;
use std::os::raw::*;
use std::net;
use std::mem;
use std::cmp;

//WinAPI Start
mod winapi {
    #![allow(bad_style)]
    #![allow(dead_code)]

    use std::os::raw::*;

    pub type SOCKET = ::std::os::windows::io::RawSocket;
    pub type DWORD = c_ulong;
    pub type WORD = c_ushort;
    pub type GROUP = c_uint;
    pub type CHAR = c_char;
    pub type USHORT = c_ushort;
    pub type ADDRESS_FAMILY = USHORT;
    pub const INVALID_SOCKET: SOCKET = !0;
    pub const SOCKET_ERROR: c_int = -1;
    pub const AF_INET: c_int = 2;
    pub const AF_INET6: c_int = 23;
    pub const WSAESHUTDOWN: DWORD = 10058;

    pub const WSADESCRIPTION_LEN: usize = 256;
    pub const WSASYS_STATUS_LEN: usize = 128;
    #[repr(C)] #[derive(Copy)]
    pub struct WSADATA {
        pub wVersion: WORD,
        pub wHighVersion: WORD,
        #[cfg(target_arch="x86")]
        pub szDescription: [c_char; WSADESCRIPTION_LEN + 1],
        #[cfg(target_arch="x86")]
        pub szSystemStatus: [c_char; WSASYS_STATUS_LEN + 1],
        pub iMaxSockets: c_ushort,
        pub iMaxUdpDg: c_ushort,
        pub lpVendorInfo: *mut c_char,
        #[cfg(target_arch="x86_64")]
        pub szDescription: [c_char; WSADESCRIPTION_LEN + 1],
        #[cfg(target_arch="x86_64")]
        pub szSystemStatus: [c_char; WSASYS_STATUS_LEN + 1],
    }

    impl Clone for WSADATA {
        fn clone(&self) -> WSADATA { *self }
    }

    #[repr(C)]
    pub struct SOCKADDR_STORAGE_LH {
        pub ss_family: ADDRESS_FAMILY,
        pub __ss_pad1: [CHAR; 6],
        pub __ss_align: i64,
        pub __ss_pad2: [CHAR; 112],
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct in_addr {
        pub s_addr: [u8; 4],
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct in6_addr {
        pub s6_addr: [u16; 8],
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct sockaddr_in {
        pub sin_family: ADDRESS_FAMILY,
        pub sin_port: USHORT,
        pub sin_addr: in_addr,
        pub sin_zero: [CHAR; 8],
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct sockaddr_in6 {
        pub sin6_family: ADDRESS_FAMILY,
        pub sin6_port: USHORT,
        pub sin6_flowinfo: c_ulong,
        pub sin6_addr: in6_addr,
        pub sin6_scope_id: c_ulong,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SOCKADDR {
        pub sa_family: ADDRESS_FAMILY,
        pub sa_data: [CHAR; 14],
    }

    pub type LPWSADATA = *mut WSADATA;

    extern "system" {
        pub fn WSAStartup(wVersionRequested: WORD, lpWSAData: LPWSADATA) -> c_int;
        pub fn WSACleanup() -> c_int;

        pub fn getsockname(s: SOCKET, name: *mut SOCKADDR, namelen: *mut c_int) -> c_int;
        pub fn socket(af: c_int, _type: c_int, protocol: c_int) -> SOCKET;
        pub fn bind(s: SOCKET, name: *const SOCKADDR, namelen: c_int) -> c_int;
        pub fn listen(s: SOCKET, backlog: c_int) -> c_int;
        pub fn accept(s: SOCKET, addr: *mut SOCKADDR, addrlen: *mut c_int) -> SOCKET;
        pub fn connect(s: SOCKET, name: *const SOCKADDR, namelen: c_int) -> c_int;
        pub fn recv(s: SOCKET, buf: *mut c_char, len: c_int, flags: c_int) -> c_int;
        pub fn recvfrom(s: SOCKET, buf: *mut c_char, len: c_int, flags: c_int, from: *mut SOCKADDR, fromlen: *mut c_int) -> c_int;
        pub fn send(s: SOCKET, buf: *const c_char, len: c_int, flags: c_int) -> c_int;
        pub fn sendto(s: SOCKET, buf: *const c_char, len: c_int, flags: c_int, to: *const SOCKADDR, tolen: c_int) -> c_int;
        pub fn getsockopt(s: SOCKET, level: c_int, optname: c_int, optval: *mut c_char, optlen: *mut c_int) -> c_int;
        pub fn setsockopt(s: SOCKET, level: c_int, optname: c_int, optval: *const c_char, optlen: c_int) -> c_int;
        pub fn ioctlsocket(s: SOCKET, cmd: c_long, argp: *mut c_ulong) -> c_int;
        pub fn shutdown(s: SOCKET, how: c_int) -> c_int;
        pub fn closesocket(s: SOCKET) -> c_int;
    }
}

use self::winapi::*;

use std::sync::{Once, ONCE_INIT};

///Raw socket
pub struct Socket {
    inner: SOCKET
}

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

impl Socket {
    ///Initializes new socket.
    ///
    ///Corresponds to C connect()
    pub fn new(family: c_int, _type: c_int, protocol: c_int) -> io::Result<Socket> {
        static INIT: Once = ONCE_INIT;

        INIT.call_once(|| {
            //just to initialize winsock inside libstd
            let _ = net::UdpSocket::bind("127.0.0.1:34254");
        });

        unsafe {
            match socket(family, _type, protocol) {
                INVALID_SOCKET => Err(io::Error::last_os_error()),
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
            let mut storage: SOCKADDR_STORAGE_LH = mem::zeroed();
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
                -1 => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == WSAESHUTDOWN as i32 {
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

    ///Receives some bytes from socket
    ///
    ///Number of received bytes and remote address are returned on success.
    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, net::SocketAddr)> {
        let len = cmp::min(buf.len(), i32::max_value() as usize) as i32;
        unsafe {
            let mut storage: SOCKADDR_STORAGE_LH = mem::zeroed();
            let mut storage_len = mem::size_of_val(&storage) as c_int;

            match recvfrom(self.inner, buf.as_mut_ptr() as *mut c_char, len, 0, &mut storage as *mut _ as *mut _, &mut storage_len) {
                -1 => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == WSAESHUTDOWN as i32 {
                        let peer_addr = sockaddr_to_addr(&storage, storage_len)?;
                        Ok((0, peer_addr))
                    }
                    else {
                        Err(error)
                    }
                },
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
                -1 => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == WSAESHUTDOWN as i32 {
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
                -1 => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == WSAESHUTDOWN as i32 {
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
            let mut storage: SOCKADDR_STORAGE_LH = mem::zeroed();
            let mut len = mem::size_of_val(&storage) as c_int;

            match accept(self.inner, &mut storage as *mut _ as *mut _, &mut len) {
                INVALID_SOCKET => Err(io::Error::last_os_error()),
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

            match ioctlsocket(self.inner, request, value) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Sets non-blocking mode.
    pub fn set_nonblocking(&self, value: bool) -> io::Result<()> {
        const FIONBIO: c_ulong = 0x8004667e;

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
            match closesocket(self.inner) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }
}

fn get_raw_addr(addr: &net::SocketAddr) -> (*const SOCKADDR, c_int) {
    match *addr {
        net::SocketAddr::V4(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as c_int)
        }
        net::SocketAddr::V6(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as c_int)
        }
    }
}

fn sockaddr_to_addr(storage: &SOCKADDR_STORAGE_LH, len: c_int) -> io::Result<net::SocketAddr> {
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
