use std::io;
use std::os::raw::*;
use std::net;
use std::mem;
use std::cmp;
use std::ptr;
use std::sync::{Once, ONCE_INIT};

mod winapi {
    #![allow(bad_style)]
    #![allow(dead_code)]

    extern crate winapi;

    pub type SOCKET = ::std::os::windows::io::RawSocket;

	pub use self::winapi::{
		ADDRESS_FAMILY,
		HANDLE,
		DWORD,
		WORD,
		GROUP,
		CHAR,
		USHORT
	};

    pub use self::winapi::{
        INVALID_SOCKET,
        SOCKET_ERROR,
        FIONBIO,

        AF_UNSPEC,
        AF_INET,
        AF_INET6,
        AF_IRDA,
        AF_BTH,

        SOCK_STREAM,
        SOCK_DGRAM,
        SOCK_RAW,
        SOCK_RDM,
        SOCK_SEQPACKET,

        IPPROTO_NONE,
        IPPROTO_ICMP,
        IPPROTO_TCP,
        IPPROTO_UDP,
        IPPROTO_ICMPV6,

        WSAESHUTDOWN,
        WSAEINVAL,

        FD_SETSIZE,
        WSADESCRIPTION_LEN,
        WSASYS_STATUS_LEN
    };

    pub const SOCK_NONBLOCK: winapi::c_int = 0o0004000;
    pub const SOCK_CLOEXEC: winapi::c_int = 0o2000000;

    pub use self::winapi::{
        WSADATA,
        fd_set,
        timeval,
        SOCKADDR_STORAGE_LH,
        in_addr,
        in6_addr,
        SOCKADDR_IN,
        sockaddr_in6,
        SOCKADDR,
        LPWSADATA
    };



    extern crate ws2_32;

    pub use self::ws2_32::{
        WSAStartup,
        WSACleanup,

        getsockname,
        socket,
        bind,
        listen,
        accept,
        connect,
        recv,
        recvfrom,
        send,
        sendto,
        getsockopt,
        setsockopt,
        ioctlsocket,
        shutdown,
        closesocket,
        select
    };


    extern crate kernel32;

    // Currently not available in `winapi`.
    pub const HANDLE_FLAG_INHERIT: winapi::DWORD = 1;

    pub use self::kernel32::{
    	SetHandleInformation,
    	GetHandleInformation
    };
}


macro_rules! impl_into_trait {
    ($($t:ty), +) => {
        $(
            impl Into<c_int> for $t {
                fn into(self) -> c_int {
                    self as c_int
                }
            }
        )+
    };
}


#[allow(non_snake_case, non_upper_case_globals)]
///Socket family
pub mod Family {
    use super::{c_int, winapi};

    pub const UNSPECIFIED: c_int = winapi::AF_UNSPEC;

    pub const IPv4: c_int = winapi::AF_INET;
    pub const IPv6: c_int = winapi::AF_INET6;
    pub const IRDA: c_int = winapi::AF_IRDA;
    pub const BTH:  c_int = winapi::AF_BTH;
}

#[allow(non_snake_case)]
///Socket type
pub mod Type {
    use super::{c_int, winapi};

    pub const STREAM:    c_int = winapi::SOCK_STREAM;
    pub const DATAGRAM:  c_int = winapi::SOCK_DGRAM;
    pub const RAW:       c_int = winapi::SOCK_RAW;
    pub const RDM:       c_int = winapi::SOCK_RDM;
    pub const SEQPACKET: c_int = winapi::SOCK_SEQPACKET;
}

#[allow(non_snake_case, non_upper_case_globals)]
///Socket protocol
pub mod Protocol {
    use super::{c_int, winapi};

    pub const NONE:   c_int = winapi::IPPROTO_NONE.0 as i32;
    pub const ICMPv4: c_int = winapi::IPPROTO_ICMP.0 as i32;
    pub const TCP:    c_int = winapi::IPPROTO_TCP.0 as i32;
    pub const UDP:    c_int = winapi::IPPROTO_UDP.0 as i32;
    pub const ICMPv6: c_int = winapi::IPPROTO_ICMPV6.0 as i32;
}

#[allow(non_snake_case)]
///Possible flags for `accept4()`
///
///Note that these flags correspond to emulated constants that are not represented
///in the OS in this way.
bitflags! (pub flags AcceptFlags: c_int {
    const NON_BLOCKING    = winapi::SOCK_NONBLOCK,
    const NON_INHERITABLE = winapi::SOCK_CLOEXEC,
});

#[repr(i32)]
#[derive(Copy, Clone)]
///Type of socket's shutdown operation.
pub enum ShutdownType {
    ///Stops any further receives.
    Receive = 0,
    ///Stops any further sends.
    Send = 1,
    ///Stops both sends and receives.
    Both = 2
}

impl_into_trait!(ShutdownType);

///Raw socket
pub struct Socket {
    inner: winapi::SOCKET
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
            match winapi::socket(family, _type, protocol) {
                winapi::INVALID_SOCKET => Err(io::Error::last_os_error()),
                fd => Ok(Socket {
                    inner: fd
                }),
            }
        }
    }

    ///Returns underlying socket descriptor.
    ///
    ///Note: ownership is not transferred.
    pub fn raw(&self) -> winapi::SOCKET {
        self.inner
    }

    ///Retrieves socket name i.e. address
    ///
    ///Wraps `getsockname()`
    ///
    ///Available for binded/connected sockets.
    pub fn name(&self) -> io::Result<net::SocketAddr> {
        unsafe {
            let mut storage: winapi::SOCKADDR_STORAGE_LH = mem::zeroed();
            let mut len = mem::size_of_val(&storage) as c_int;

            match winapi::getsockname(self.inner, &mut storage as *mut _ as *mut _, &mut len) {
                winapi::SOCKET_ERROR => Err(io::Error::last_os_error()),
                _ => sockaddr_to_addr(&storage, len)
            }
        }
    }

    ///Binds socket to address.
    pub fn bind(&self, addr: &net::SocketAddr) -> io::Result<()> {
        let (addr, len) = get_raw_addr(addr);

        unsafe {
            match winapi::bind(self.inner, addr, len) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Listens for incoming connections on this socket.
    pub fn listen(&self, backlog: c_int) -> io::Result<()> {
        unsafe {
            match winapi::listen(self.inner, backlog) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Receives some bytes from socket
    ///
    ///Number of received bytes is returned on success
    pub fn recv(&self, buf: &mut [u8], flags: c_int) -> io::Result<usize> {
        let len = cmp::min(buf.len(), i32::max_value() as usize) as i32;
        unsafe {
            match winapi::recv(self.inner, buf.as_mut_ptr() as *mut c_char, len, flags) {
                -1 => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == winapi::WSAESHUTDOWN as i32 {
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
    pub fn recv_from(&self, buf: &mut [u8], flags: c_int) -> io::Result<(usize, net::SocketAddr)> {
        let len = cmp::min(buf.len(), i32::max_value() as usize) as i32;
        unsafe {
            let mut storage: winapi::SOCKADDR_STORAGE_LH = mem::zeroed();
            let mut storage_len = mem::size_of_val(&storage) as c_int;

            match winapi::recvfrom(self.inner, buf.as_mut_ptr() as *mut c_char, len, flags, &mut storage as *mut _ as *mut _, &mut storage_len) {
                -1 => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == winapi::WSAESHUTDOWN as i32 {
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
    pub fn send(&self, buf: &[u8], flags: c_int) -> io::Result<usize> {
        let len = cmp::min(buf.len(), i32::max_value() as usize) as i32;

        unsafe {
            match winapi::send(self.inner, buf.as_ptr() as *const c_char, len, flags) {
                -1 => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == winapi::WSAESHUTDOWN as i32 {
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
    pub fn send_to(&self, buf: &[u8], peer_addr: &net::SocketAddr, flags: c_int) -> io::Result<usize> {
        let len = cmp::min(buf.len(), i32::max_value() as usize) as i32;
        let (addr, addr_len) = get_raw_addr(peer_addr);

        unsafe {
            match winapi::sendto(self.inner, buf.as_ptr() as *const c_char, len, flags, addr, addr_len) {
                -1 => {
                    let error = io::Error::last_os_error();
                    let raw_code = error.raw_os_error().unwrap();

                    if raw_code == winapi::WSAESHUTDOWN as i32 {
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

    ///Accept a new incoming client connection and return its files descriptor and address.
    ///
    ///This is an emulation of the corresponding Unix system call, that will automatically call
    ///`.set_blocking` and `.set_inheritable` with parameter values based on the value of `flags`
    ///on the created client socket:
    ///
    /// * `AcceptFlags::NON_BLOCKING`    – Mark the newly created socket as non-blocking
    /// * `AcceptFlags::NON_INHERITABLE` – Mark the newly created socket as not inheritable by client processes
    pub fn accept4(&self, flags: AcceptFlags) -> io::Result<(Socket, net::SocketAddr)> {
        self.accept().map(|(sock, addr)| {
            // Emulate the two most common (and useful) `accept4` flags
            sock.set_blocking(!flags.contains(NON_BLOCKING)).expect("Setting newly obtained client socket blocking mode");
            sock.set_inheritable(!flags.contains(NON_INHERITABLE)).expect("Setting newly obtained client socket inheritance mode");

            (sock, addr)
        })
    }

    ///Accepts incoming connection.
    pub fn accept(&self) -> io::Result<(Socket, net::SocketAddr)> {
        unsafe {
            let mut storage: winapi::SOCKADDR_STORAGE_LH = mem::zeroed();
            let mut len = mem::size_of_val(&storage) as c_int;

            match winapi::accept(self.inner, &mut storage as *mut _ as *mut _, &mut len) {
                winapi::INVALID_SOCKET => Err(io::Error::last_os_error()),
                sock @ _ => {
                    let addr = sockaddr_to_addr(&storage, len)?;
                    Ok((Socket { inner: sock }, addr))
                }
            }
        }
    }

    ///Connects socket with remote address.
    pub fn connect(&self, addr: &net::SocketAddr) -> io::Result<()> {
        let (addr, len) = get_raw_addr(addr);

        unsafe {
            match winapi::connect(self.inner, addr, len) {
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

            match winapi::getsockopt(self.inner, level, name, value_ptr, &mut value_len) {
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

            match winapi::setsockopt(self.inner, level, name, value, mem::size_of::<T>() as c_int) {
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

            match winapi::ioctlsocket(self.inner, request, value) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Sets non-blocking mode.
    pub fn set_blocking(&self, value: bool) -> io::Result<()> {
        self.ioctl(winapi::FIONBIO as c_int, (!value) as c_ulong)
    }


    ///Sets whether this socket will be inherited by child processes or not.
    ///
    ///Internally this implemented by calling `SetHandleInformation(sock, HANDLE_FLAG_INHERIT, …)`.
    pub fn set_inheritable(&self, value: bool) -> io::Result<()> {
        unsafe {
            let flag = if value { winapi::HANDLE_FLAG_INHERIT } else { 0 };
            match winapi::SetHandleInformation(self.inner as winapi::HANDLE, winapi::HANDLE_FLAG_INHERIT, flag) {
                0 => Err(io::Error::last_os_error()),
                _ => Ok(())
            }
        }
    }


	///Returns whether this socket will be inherited by child processes or not.
	pub fn get_inheritable(&self) -> io::Result<bool> {
		unsafe {
			let mut flags: winapi::DWORD = 0;
			match winapi::GetHandleInformation(self.inner as winapi::HANDLE, &mut flags as *mut _) {
                0 => Err(io::Error::last_os_error()),
                _ => Ok((flags & winapi::HANDLE_FLAG_INHERIT) != 0)
            }
        }
	}


    ///Stops receive and/or send over socket.
    pub fn shutdown(&self, direction: ShutdownType) -> io::Result<()> {
        unsafe {
            match winapi::shutdown(self.inner, direction.into()) {
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
            match winapi::closesocket(self.inner) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }
}

fn get_raw_addr(addr: &net::SocketAddr) -> (*const winapi::SOCKADDR, c_int) {
    match *addr {
        net::SocketAddr::V4(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as c_int)
        }
        net::SocketAddr::V6(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as c_int)
        }
    }
}

fn sockaddr_to_addr(storage: &winapi::SOCKADDR_STORAGE_LH, len: c_int) -> io::Result<net::SocketAddr> {
    match storage.ss_family as c_int {
        winapi::AF_INET => {
            assert!(len as usize >= mem::size_of::<winapi::SOCKADDR_IN>());
            let storage = unsafe { *(storage as *const _ as *const winapi::SOCKADDR_IN) };
            let address = unsafe { storage.sin_addr.S_un_b() };
            let ip = net::Ipv4Addr::new(address.s_b1,
                                        address.s_b2,
                                        address.s_b3,
                                        address.s_b4);

            //Note to_be() swap bytes on LE targets
            //As IP stuff is always BE, we need swap only on LE targets
            Ok(net::SocketAddr::V4(net::SocketAddrV4::new(ip, storage.sin_port.to_be())))
        }
        winapi::AF_INET6 => {
            assert!(len as usize >= mem::size_of::<winapi::sockaddr_in6>());
            let storage = unsafe { *(storage as *const _ as *const winapi::sockaddr_in6) };
            let ip = net::Ipv6Addr::from(storage.sin6_addr.s6_addr.clone());

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

use std::os::windows::io::{
    AsRawSocket,
    FromRawSocket,
    IntoRawSocket,
};

impl AsRawSocket for Socket {
    fn as_raw_socket(&self) -> winapi::SOCKET {
        self.inner
    }
}

impl FromRawSocket for Socket {
    unsafe fn from_raw_socket(sock: winapi::SOCKET) -> Self {
        Socket {inner: sock}
    }
}

impl IntoRawSocket for Socket {
    fn into_raw_socket(self) -> winapi::SOCKET {
        let result = self.inner;
        mem::forget(self);
        result
    }
}

#[inline]
fn ms_to_timeval(timeout_ms: u64) -> winapi::timeval {
    winapi::timeval {
        tv_sec: timeout_ms as c_long / 1000,
        tv_usec: (timeout_ms as c_long % 1000) * 1000
    }
}

fn sockets_to_fd_set(sockets: &[&Socket]) -> winapi::fd_set {
    assert!(sockets.len() < winapi::FD_SETSIZE);
    let mut raw_fds: winapi::fd_set = unsafe { mem::zeroed() };

    for socket in sockets {
        let idx = raw_fds.fd_count as usize;
        raw_fds.fd_array[idx] = socket.inner;
        raw_fds.fd_count += 1;
    }

    raw_fds
}

///Wrapper over system `select`
///
///Returns number of sockets that are ready.
///
///If timeout isn't specified then select will be blocking call.
///
///## Note:
///
///Number of each set cannot be bigger than FD_SETSIZE i.e. 64
///
///## Warning:
///
///It is invalid to pass all sets of descriptors empty on Windows.
pub fn select(read_fds: &[&Socket], write_fds: &[&Socket], except_fds: &[&Socket], timeout_ms: Option<u64>) -> io::Result<c_int> {
    let mut raw_read_fds = sockets_to_fd_set(read_fds);
    let mut raw_write_fds = sockets_to_fd_set(write_fds);
    let mut raw_except_fds = sockets_to_fd_set(except_fds);

    unsafe {
        match winapi::select(0,
                             if read_fds.len() > 0 { &mut raw_read_fds } else { ptr::null_mut() },
                             if write_fds.len() > 0 { &mut raw_write_fds } else { ptr::null_mut() },
                             if except_fds.len() > 0 { &mut raw_except_fds } else { ptr::null_mut() },
                             if let Some(timeout_ms) = timeout_ms { &ms_to_timeval(timeout_ms) } else { ptr::null() } ) {
            winapi::SOCKET_ERROR => Err(io::Error::last_os_error()),
            result @ _ => Ok(result)

        }
    }
}
