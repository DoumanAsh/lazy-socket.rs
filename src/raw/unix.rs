use std::net;
use std::io;
use std::mem;
use std::ptr;
use std::cmp;

mod libc {
    extern crate libc;

    //Types
    pub use self::libc::{
        c_int,
        c_void,
        c_char,
        c_long,
        c_ulong,
        ssize_t,
        socklen_t,
        size_t,
        sockaddr,
        sockaddr_storage,
        sa_family_t,
        in_port_t,
        fd_set,
        timeval,
        time_t,
        suseconds_t
    };

	pub use self::libc::{
        sockaddr_in,
        sockaddr_in6,

        in_addr,
        in6_addr
    };

    pub type SOCKET = c_int;
    pub const SOCKET_ERROR: c_int = -1;
    pub const SOCKET_SHUTDOWN: c_int = libc::ESHUTDOWN;

    //Constants
    pub use self::libc::{
        EINVAL,
        FIONBIO,
        F_GETFD,
        F_SETFD,
        FD_CLOEXEC
    };

    #[cfg(target_os = "macos")]
    pub use self::libc::{
        AF_UNIX,
        AF_INET,
        AF_INET6,
        SOCK_STREAM,
        SOCK_DGRAM,
        SOCK_RAW,
        SOCK_SEQPACKET,
    };

    #[cfg(target_os = "macos")]
    pub const AF_UNSPEC: c_int = 0;
    #[cfg(target_os = "macos")]
    pub const SOCK_NONBLOCK: c_int = 0o0004000;
    #[cfg(target_os = "macos")]
    pub const SOCK_CLOEXEC: c_int = 0o2000000;

    #[cfg(not(target_os = "macos"))]
    pub use self::libc::{
        AF_UNSPEC,
        AF_UNIX,
        AF_INET,
        AF_INET6,
        AF_NETLINK,
        AF_PACKET,
        SOCK_STREAM,
        SOCK_DGRAM,
        SOCK_RAW,
        SOCK_SEQPACKET,
        SOCK_NONBLOCK,
        SOCK_CLOEXEC
    };

    //Functions
    pub use self::libc::{
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
        fcntl,
        ioctl,
        shutdown,
        close,
        select,
        FD_SET
    };

    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "dragonflybsd"))]
    pub use self::libc::{
        accept4
    };
}

use self::libc::*;

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
    use super::libc::*;
    pub const UNSPECIFIED: c_int = AF_UNSPEC;
    pub const UNIX: c_int = AF_UNIX;
    pub const IPv4: c_int = AF_INET;
    pub const IPv6: c_int = AF_INET6;
    #[cfg(not(target_os = "macos"))]
    pub const NETLINK: c_int = AF_NETLINK;
    #[cfg(not(target_os = "macos"))]
    pub const PACKET: c_int = AF_PACKET;
}

#[allow(non_snake_case)]
///Socket type
pub mod Type {
    use super::libc::*;
    pub const STREAM: c_int = SOCK_STREAM;
    pub const DATAGRAM: c_int = SOCK_DGRAM;
    pub const RAW: c_int = SOCK_RAW;
    pub const SEQPACKET: c_int = SOCK_SEQPACKET;
    #[cfg(not(target_os = "macos"))]
    ///Applied through bitwise OR
    pub const NONBLOCK: c_int = SOCK_NONBLOCK;
    #[cfg(not(target_os = "macos"))]
    ///Applied through bitwise OR
    pub const CLOEXEC: c_int = SOCK_CLOEXEC;
}

#[allow(non_snake_case, non_upper_case_globals)]
///Socket protocol
pub mod Protocol {
    use super::libc::*;
    pub const NONE: c_int = 0;
    pub const ICMPv4: c_int = 1;
    pub const TCP: c_int = 6;
    pub const UDP: c_int = 17;
    pub const ICMPv6: c_int = 58;
}

#[allow(non_snake_case)]
///Possible flags for `accept4()`
bitflags! (pub flags AcceptFlags: c_int {
    const NON_BLOCKING    = SOCK_NONBLOCK,
    const NON_INHERITABLE = SOCK_CLOEXEC,
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
            let mut len = mem::size_of_val(&storage) as socklen_t;

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
    pub fn recv(&self, buf: &mut [u8], flags: c_int) -> io::Result<usize> {
        let len = buf.len();

        unsafe {
            match recv(self.inner, buf.as_mut_ptr() as *mut c_void, len, flags) {
                -1 => Err(io::Error::last_os_error()),
                n => Ok(n as usize)
            }
        }
    }

    ///Receives some bytes from socket
    ///
    ///Number of received bytes and remote address are returned on success.
    pub fn recv_from(&self, buf: &mut [u8], flags: c_int) -> io::Result<(usize, net::SocketAddr)> {
        let len = buf.len();

        unsafe {
            let mut storage: sockaddr_storage = mem::zeroed();
            let mut storage_len = mem::size_of_val(&storage) as socklen_t;

            match recvfrom(self.inner, buf.as_mut_ptr() as *mut c_void, len, flags, &mut storage as *mut _ as *mut _, &mut storage_len) {
                -1 => Err(io::Error::last_os_error()),
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
        let len = buf.len();

        unsafe {
            match send(self.inner, buf.as_ptr() as *const c_void, len, flags) {
                -1 => {
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
    pub fn send_to(&self, buf: &[u8], peer_addr: &net::SocketAddr, flags: c_int) -> io::Result<usize> {
        let len = buf.len();
        let (addr, addr_len) = get_raw_addr(peer_addr);

        unsafe {
            match sendto(self.inner, buf.as_ptr() as *const c_void, len, flags, addr, addr_len) {
                -1 => {
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

    ///Accept a new incoming client connection and return its files descriptor and address.
    ///
    ///By default the newly created socket will be inheritable by child processes and created
    ///in blocking I/O mode. This behaviour can be customized using the `flags` parameter:
    ///
    /// * `AcceptFlags::NON_BLOCKING`    – Mark the newly created socket as non-blocking
    /// * `AcceptFlags::NON_INHERITABLE` – Mark the newly created socket as not inheritable by client processes
    ///
    ///Depending on the operating system's availablility of the `accept4(2)` system call this call
    ///either pass the flags on to the operating system or emulate the call using `accept(2)`.
    pub fn accept4(&self, flags: AcceptFlags) -> io::Result<(Socket, net::SocketAddr)> {
        #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "dragonflybsd"))]
        unsafe {
            let mut storage: sockaddr_storage = mem::zeroed();
            let mut len = mem::size_of_val(&storage) as socklen_t;

            match accept4(self.inner, &mut storage as *mut _ as *mut _, &mut len, flags.bits()) {
                SOCKET_ERROR => Err(io::Error::last_os_error()),
                sock @ _ => {
                    let addr = sockaddr_to_addr(&storage, len)?;
                    Ok((Socket { inner: sock, }, addr))
                }
            }
        }

        #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "dragonflybsd")))]
        {
            self.accept().map(|(sock, addr)| {
                // Emulate the two most common (and useful) `accept4` flags using `ioctl`/`fcntl`
                //
                // The only errors that can happen here fall into two categories:
                //
                //  * Programming errors on our side
                //    (unlikely, but in this case panicing is actually the right thing to do anyway)
                //  * Another thread causing havok with random file descriptors
                //    (always very bad and nothing, particularily since there is absolutely nothing
                //     that we OR USER can do about this)
                sock.set_blocking(!flags.contains(NON_BLOCKING)).expect("Setting newly obtained client socket blocking mode");
                sock.set_inheritable(!flags.contains(NON_INHERITABLE)).expect("Setting newly obtained client socket inheritance mode");

                (sock, addr)
            })
        }
    }


    ///Accept a new incoming client connection and return its files descriptor and address.
    ///
    ///As this uses the classic `accept(2)` system call internally, you are **strongly advised** to
    ///use the `.accept4()` method instead to get definied blocking and inheritance semantics for
    ///the created file descriptor.
    pub fn accept(&self) -> io::Result<(Socket, net::SocketAddr)> {
        unsafe {
            let mut storage: sockaddr_storage = mem::zeroed();
            let mut len = mem::size_of_val(&storage) as socklen_t;

            match accept(self.inner, &mut storage as *mut _ as *mut _, &mut len) {
                SOCKET_ERROR => Err(io::Error::last_os_error()),
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
            let value_ptr = &mut value as *mut T as *mut c_void;
            let mut value_len = mem::size_of::<T>() as socklen_t;

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
            let value = &value as *const T as *const c_void;

            match setsockopt(self.inner, level, name, value, mem::size_of::<T>() as socklen_t) {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error())
            }
        }
    }

    ///Sets I/O parameters of socket.
    pub fn ioctl(&self, request: c_ulong, value: c_ulong) -> io::Result<()> {
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
    pub fn set_blocking(&self, value: bool) -> io::Result<()> {
        self.ioctl(FIONBIO, (!value) as c_ulong)
    }


    ///Sets whether this socket will be inherited by newly created processes or not.
    ///
    ///Internally this is implemented by calling `fcntl(fd, F_GETFD)` and `fcntl(fd, F_SETFD)`
    ///to update the `FD_CLOEXEC` flag. (In the future this might use `ioctl(2)` on some
    ///platforms instead.)
    ///
    ///This means that the socket will still be available to forked off child processes until it
    ///calls `execve(2)` to complete the creation of a new process. A forking server application
    ///(or similar) should therefor not expect this flag to have any effect on spawned off workers;
    ///you're advised to manually call `.close()` on the socket instance in the worker process
    ///instead. The standard library's `std::process` facility is not impacted by this however.
    pub fn set_inheritable(&self, value: bool) -> io::Result<()> {
        // Some (or possibly all?) OS's support the `FIOCLEX` and `FIONCLEX`
        // `ioctl`s instead, however there is no support for that in `libc`
        // currently and no usable documentation for figuring out who supports
        // this feature online either
        unsafe {
            let mut flags: libc::c_int = libc::fcntl(self.inner, libc::F_GETFD);
            if flags < 0 {
                return Err(io::Error::last_os_error());
            }

            if value == true {
                flags &= !libc::FD_CLOEXEC;
            } else {
                flags |= libc::FD_CLOEXEC;
            }

            if libc::fcntl(self.inner, libc::F_SETFD, flags) < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        Ok(())
    }

	///Returns whether this will be inherited by newly created processes or not.
	///
	///See `set_inheritable` for a detailed description of what this means.
	pub fn get_inheritable(&self) -> io::Result<bool> {
		unsafe {
            let flags = libc::fcntl(self.inner, libc::F_GETFD);
            if flags < 0 {
                return Err(io::Error::last_os_error());
            }

            Ok((flags & libc::FD_CLOEXEC) == 0)
        }
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

fn get_raw_addr(addr: &net::SocketAddr) -> (*const sockaddr, socklen_t) {
    match *addr {
        net::SocketAddr::V4(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as socklen_t)
        }
        net::SocketAddr::V6(ref a) => {
            (a as *const _ as *const _, mem::size_of_val(a) as socklen_t)
        }
    }
}

fn sockaddr_to_addr(storage: &sockaddr_storage, len: socklen_t) -> io::Result<net::SocketAddr> {
    match storage.ss_family as c_int {
        AF_INET => {
            assert!(len as usize >= mem::size_of::<sockaddr_in>());
            let storage = unsafe { *(storage as *const _ as *const sockaddr_in) };
            let address = unsafe { *(&storage.sin_addr.s_addr as *const _ as *const [u8; 4]) };
            let ip = net::Ipv4Addr::from(address);

            //Note to_be() swap bytes on LE targets
            //As IP stuff is always BE, we need swap only on LE targets
            Ok(net::SocketAddr::V4(net::SocketAddrV4::new(ip, storage.sin_port.to_be())))
        }
        AF_INET6 => {
            assert!(len as usize >= mem::size_of::<sockaddr_in6>());
            let storage = unsafe { *(storage as *const _ as *const sockaddr_in6) };
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

use std::os::unix::io::{
    AsRawFd,
    FromRawFd,
    IntoRawFd,
};

impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> SOCKET {
        self.inner
    }
}

impl FromRawFd for Socket {
    unsafe fn from_raw_fd(sock: SOCKET) -> Self {
        Socket {inner: sock}
    }
}

impl IntoRawFd for Socket {
    fn into_raw_fd(self) -> SOCKET {
        let result = self.inner;
        mem::forget(self);
        result
    }
}

#[inline]
fn ms_to_timeval(timeout_ms: u64) -> timeval {
    timeval {
        tv_sec: timeout_ms as time_t / 1000,
        tv_usec: (timeout_ms as suseconds_t % 1000) * 1000
    }
}

fn sockets_to_fd_set(sockets: &[&Socket]) -> (c_int, fd_set) {
    let mut max_fd: c_int = 0;
    let mut raw_fds: fd_set = unsafe { mem::zeroed() };

    for socket in sockets {
        max_fd = cmp::max(max_fd, socket.inner);
        unsafe {
            FD_SET(socket.inner, &mut raw_fds);
        }
    }

    (max_fd, raw_fds)
}

///Wrapper over system `select`
///
///Returns number of sockets that are ready.
///
///If timeout isn't specified then select will be a blocking call.
pub fn select(read_fds: &[&Socket], write_fds: &[&Socket], except_fds: &[&Socket], timeout_ms: Option<u64>) -> io::Result<c_int> {
    let (max_read_fd, mut raw_read_fds) = sockets_to_fd_set(read_fds);
    let (max_write_fd, mut raw_write_fds) = sockets_to_fd_set(write_fds);
    let (max_except_fd, mut raw_except_fds) = sockets_to_fd_set(except_fds);

    let nfds = cmp::max(max_read_fd, cmp::max(max_write_fd, max_except_fd)) + 1;

    unsafe {
        match libc::select(nfds,
                           if max_read_fd > 0 { &mut raw_read_fds } else { ptr::null_mut() },
                           if max_write_fd > 0 { &mut raw_write_fds } else { ptr::null_mut() },
                           if max_except_fd > 0 { &mut raw_except_fds } else { ptr::null_mut() },
                           if let Some(timeout_ms) = timeout_ms { &mut ms_to_timeval(timeout_ms) } else { ptr::null_mut() } ) {
            SOCKET_ERROR => Err(io::Error::last_os_error()),
            result @ _ => Ok(result)

        }
    }
}
