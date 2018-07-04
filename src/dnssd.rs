use libc::{
    c_char, c_int, c_uchar, c_void, int32_t, sa_family_t, sockaddr, sockaddr_in, sockaddr_in6,
    uint16_t, uint32_t, AF_INET,
};
use mio;
use mio::unix::EventedFd;
use std::convert::From;
use std::ffi::{CStr, CString};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ptr;
use std::sync::Mutex;
use tokio::prelude::*;
use tokio::reactor::PollEvented2;

pub enum DNSService {}

pub type DNSServiceRef = *mut DNSService;

#[derive(Debug)]
pub struct BoxedDNSServiceRef(DNSServiceRef);

unsafe impl Send for BoxedDNSServiceRef {}

impl Drop for BoxedDNSServiceRef {
    fn drop(&mut self) {
        dns_service_ref_deallocate(self.0);
    }
}

type DNSServiceFlags = uint32_t;

pub const DNS_SERVICE_FLAGS_FORCE_MULTICAST: DNSServiceFlags = 0x400;

#[allow(non_camel_case_types)]
pub type dnssd_sock_t = c_int;

pub type DNSServiceErrorType = int32_t;

type DNSServiceProtocol = uint32_t;

pub const DNS_SERVICE_PROTOCOL_IPV4: DNSServiceProtocol = 0x01;

type DNSServiceRegisterReply = extern "C" fn(
    sd_ref: DNSServiceRef,
    flags: DNSServiceFlags,
    error_code: int32_t,
    name: *const c_char,
    regtype: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
);

type DNSServiceBrowseReply = extern "C" fn(
    sd_ref: DNSServiceRef,
    flags: DNSServiceFlags,
    interface_index: uint32_t,
    error_code: DNSServiceErrorType,
    service_name: *const c_char,
    regtype: *const c_char,
    reply_domain: *const c_char,
    context: *mut c_void,
);

type DNSServiceResolveReply = extern "C" fn(
    sd_ref: DNSServiceRef,
    flags: uint32_t,
    interface_index: uint32_t,
    error_code: DNSServiceErrorType,
    fullname: *const c_char,
    hosttarget: *const c_char,
    port: uint16_t,
    txt_len: uint16_t,
    txt_record: *const c_uchar,
    context: *mut c_void,
);

type DNSServiceGetAddrInfoReply = extern "C" fn(
    sd_ref: DNSServiceRef,
    flags: DNSServiceFlags,
    interface_index: uint32_t,
    error_code: DNSServiceErrorType,
    hostname: *const c_char,
    address: *const sockaddr,
    ttl: uint32_t,
    context: *mut c_void,
);

extern "C" fn dns_service_register_cb(
    _sd_ref: DNSServiceRef,
    _flags: uint32_t,
    error_code: DNSServiceErrorType,
    name: *const c_char,
    regtype: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
) {
    let service_result_mutex: &mut Mutex<Result<Service, ServiceError>> =
        unsafe { &mut *(context as *mut Mutex<Result<Service, ServiceError>>) };
    let err = ServiceError::from(error_code);
    let mut service_guard = service_result_mutex.lock().unwrap();
    *service_guard = if let ServiceError::NoError = err {
        unsafe {
            Ok(Service {
                name: CStr::from_ptr(name).to_string_lossy().into_owned(),
                regtype: CStr::from_ptr(regtype).to_string_lossy().into_owned(),
                domain: CStr::from_ptr(domain).to_string_lossy().into_owned(),
            })
        }
    } else {
        Err(err)
    };
}

pub fn dns_service_register(
    service_result_mutex: &mut Mutex<Result<Service, ServiceError>>,
) -> Result<BoxedDNSServiceRef, ServiceError> {
    let reg_type = CString::new("_localchat._tcp.").unwrap();
    let context = service_result_mutex as *mut _ as *mut c_void;
    unsafe {
        let mut sd_ref: DNSServiceRef = ptr::null_mut();
        let sd_ref_ptr = &mut sd_ref as *mut DNSServiceRef;
        let err = DNSServiceRegister(
            sd_ref_ptr,
            0,
            0,
            ptr::null(),
            reg_type.as_ptr(),
            ptr::null(),
            ptr::null(),
            1337,
            0,
            ptr::null(),
            dns_service_register_cb,
            context,
        );
        let err = ServiceError::from(err);
        if let ServiceError::NoError = err {
            Ok(BoxedDNSServiceRef(sd_ref))
        } else {
            Err(err)
        }
    }
}

extern "C" fn dns_service_browse_cb(
    _sd_ref: DNSServiceRef,
    flags: DNSServiceFlags,
    _interface_index: uint32_t,
    error_code: DNSServiceErrorType,
    name: *const c_char,
    regtype: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
) {
    let browse_event_mutex: &mut Mutex<Result<BrowseEvent, ServiceError>> =
        unsafe { &mut *(context as *mut Mutex<Result<BrowseEvent, ServiceError>>) };
    let mut browse_guard = browse_event_mutex.lock().unwrap();
    let err = ServiceError::from(error_code);
    *browse_guard = if let ServiceError::NoError = err {
        let service = unsafe {
            Service {
                name: CStr::from_ptr(name).to_string_lossy().into_owned(),
                regtype: CStr::from_ptr(regtype).to_string_lossy().into_owned(),
                domain: CStr::from_ptr(domain).to_string_lossy().into_owned(),
            }
        };
        let browse_event = if flags & 0x2 > 0 {
            BrowseEvent::Joined(service)
        } else {
            BrowseEvent::Dropped(service)
        };
        Ok(browse_event)
    } else {
        Err(err)
    };
}

pub fn dns_service_browse(
    browse_event_result: &mut Mutex<Result<BrowseEvent, ServiceError>>,
) -> Result<BoxedDNSServiceRef, ServiceError> {
    let context = browse_event_result as *mut _ as *mut c_void;
    unsafe {
        let mut sd_ref: DNSServiceRef = ptr::null_mut();
        let sd_ref_ptr = &mut sd_ref as *mut DNSServiceRef;
        let reg_type = CString::new("_localchat._tcp.").unwrap();
        let err = DNSServiceBrowse(
            sd_ref_ptr,
            0,
            0,
            reg_type.as_ptr(),
            ptr::null(),
            dns_service_browse_cb,
            context,
        );
        let err = ServiceError::from(err);
        if let ServiceError::NoError = err {
            Ok(BoxedDNSServiceRef(sd_ref))
        } else {
            Err(err)
        }
    }
}

extern "C" fn dns_service_resolve_reply(
    _sd_ref: DNSServiceRef,
    _flags: uint32_t,
    _interface_index: uint32_t,
    error_code: DNSServiceErrorType,
    _fullname: *const c_char,
    hosttarget: *const c_char,
    port: uint16_t,
    _txt_len: uint16_t,
    _txt_record: *const c_uchar,
    context: *mut c_void,
) {
    let port = u16::from_be(port); // Note that `port` is in network byte order (big-endian)
    let name = unsafe { CStr::from_ptr(hosttarget).to_string_lossy().into_owned() };
    let host = Host { name, port };
    let err = ServiceError::from(error_code);
    let host_result_mutex: &mut Mutex<Result<Host, ServiceError>> =
        unsafe { &mut *(context as *mut Mutex<Result<Host, ServiceError>>) };
    let mut guard = host_result_mutex.lock().unwrap();
    *guard = if let ServiceError::NoError = err {
        Ok(host)
    } else {
        Err(err)
    };
}

pub fn dns_service_resolve(
    service: &Service,
    host: &mut Mutex<Result<Host, ServiceError>>,
) -> Result<BoxedDNSServiceRef, ServiceError> {
    let flags = DNS_SERVICE_FLAGS_FORCE_MULTICAST;
    let name = CString::new(service.name.as_bytes()).unwrap().into_raw();
    let regtype = CString::new(service.regtype.as_bytes()).unwrap().into_raw();
    let domain = CString::new(service.domain.as_bytes()).unwrap().into_raw();
    let context = host as *mut _ as *mut c_void;
    unsafe {
        let mut sd_ref: DNSServiceRef = ptr::null_mut();
        let sd_ref_ptr = &mut sd_ref as *mut DNSServiceRef;
        let err = DNSServiceResolve(
            sd_ref_ptr,
            flags,
            0,
            name,
            regtype,
            domain,
            dns_service_resolve_reply,
            context,
        );
        let err = ServiceError::from(err);
        if let ServiceError::NoError = err {
            Ok(BoxedDNSServiceRef(sd_ref))
        } else {
            Err(err)
        }
    }
}

extern "C" fn dns_service_get_addr_info_reply(
    _sd_ref: DNSServiceRef,
    _flags: DNSServiceFlags,
    _interface_index: uint32_t,
    error_code: DNSServiceErrorType,
    _hostname: *const c_char,
    address: *const sockaddr,
    _ttl: uint32_t,
    context: *mut c_void,
) {
    let err = ServiceError::from(error_code);
    let result = if let ServiceError::NoError = err {
        let addr = unsafe {
            if (*address).sa_family == AF_INET as sa_family_t {
                // IPv4
                let s_addr = *(address as *const sockaddr_in);
                let addr = u32::from_be(s_addr.sin_addr.s_addr);
                IpAddr::V4(Ipv4Addr::new(
                    ((addr >> 24) & 0xff) as u8,
                    ((addr >> 16) & 0xff) as u8,
                    ((addr >> 8) & 0xff) as u8,
                    (addr & 0xff) as u8,
                ))
            } else {
                // IPv6
                let s_addr: sockaddr_in6 = *(address as *const sockaddr_in6);
                let s6_addr = s_addr.sin6_addr.s6_addr;
                let mut chunks = s6_addr
                    .chunks(2)
                    .map(|tuple| ((tuple[0] as u16) << 8) | (tuple[1] as u16));
                let a = chunks.next().unwrap();
                let b = chunks.next().unwrap();
                let c = chunks.next().unwrap();
                let d = chunks.next().unwrap();
                let e = chunks.next().unwrap();
                let f = chunks.next().unwrap();
                let g = chunks.next().unwrap();
                let h = chunks.next().unwrap();
                IpAddr::V6(Ipv6Addr::new(a, b, c, d, e, f, g, h))
            }
        };
        Ok(addr)
    } else {
        Err(err)
    };
    let ipaddr_mutex: &mut Mutex<Result<IpAddr, ServiceError>> =
        unsafe { &mut *(context as *mut Mutex<Result<IpAddr, ServiceError>>) };
    let mut guard = ipaddr_mutex.lock().unwrap();
    *guard = result;
}

pub fn dns_service_get_addr_info(
    host: &Host,
    ipaddr_mutex: &mut Mutex<Result<IpAddr, ServiceError>>,
) -> Result<BoxedDNSServiceRef, ServiceError> {
    let mut sd_ref: DNSServiceRef = ptr::null_mut();
    let sd_ref_ptr = &mut sd_ref as *mut DNSServiceRef;
    let hostname = CString::new(host.name.as_bytes()).unwrap().into_raw();
    let context = ipaddr_mutex as *mut _ as *mut c_void;
    let err = unsafe {
        DNSServiceGetAddrInfo(
            sd_ref_ptr,
            0,
            0,
            // could change to "not care" about v4 or v6
            DNS_SERVICE_PROTOCOL_IPV4,
            hostname,
            dns_service_get_addr_info_reply,
            context,
        )
    };
    let err = ServiceError::from(err);
    if let ServiceError::NoError = err {
        Ok(BoxedDNSServiceRef(sd_ref))
    } else {
        Err(err)
    }
}

pub fn dns_service_ref_socket(sd_ref: &BoxedDNSServiceRef) -> Result<dnssd_sock_t, ServiceError> {
    let sock_fd = unsafe { DNSServiceRefSockFD(sd_ref.0) };
    if sock_fd == -1 {
        Err(ServiceError::Unknown)
    } else {
        Ok(sock_fd)
    }
}

pub fn dns_service_process_result(sd_ref: &BoxedDNSServiceRef) -> Result<(), ServiceError> {
    let err = unsafe { ServiceError::from(DNSServiceProcessResult(sd_ref.0)) };
    if let ServiceError::NoError = err {
        Ok(())
    } else {
        Err(err)
    }
}

pub fn dns_service_ref_deallocate(sd_ref: DNSServiceRef) {
    unsafe { DNSServiceRefDeallocate(sd_ref) };
}

extern "C" {
    fn DNSServiceBrowse(
        sd_ref: *mut DNSServiceRef,
        flags: DNSServiceFlags,
        interface_index: uint32_t,
        regtype: *const c_char,
        domain: *const c_char,
        callBack: DNSServiceBrowseReply,
        context: *mut c_void,
    ) -> DNSServiceErrorType;

    fn DNSServiceRegister(
        sd_ref: *mut DNSServiceRef,
        flags: uint32_t,
        interface_index: uint32_t,
        name: *const c_char,
        reg_type: *const c_char,
        domain: *const c_char,
        host: *const c_char,
        port: uint16_t,
        txt_len: uint16_t,
        txt_record: *const c_void,
        callback: DNSServiceRegisterReply,
        context: *mut c_void,
    ) -> DNSServiceErrorType;

    fn DNSServiceResolve(
        sd_ref: *mut DNSServiceRef,
        flags: DNSServiceFlags,
        interface_index: uint32_t,
        name: *const c_char,
        regtype: *const c_char,
        domain: *const c_char,
        callback: DNSServiceResolveReply,
        context: *mut c_void,
    ) -> DNSServiceErrorType;

    fn DNSServiceGetAddrInfo(
        sd_ref: *mut DNSServiceRef,
        flags: DNSServiceFlags,
        interface_index: uint32_t,
        protocol: DNSServiceProtocol,
        hostname: *const c_char,
        callback: DNSServiceGetAddrInfoReply,
        context: *mut c_void,
    ) -> DNSServiceErrorType;

    fn DNSServiceRefSockFD(sd_ref: DNSServiceRef) -> dnssd_sock_t;

    fn DNSServiceProcessResult(sd_ref: DNSServiceRef) -> DNSServiceErrorType;

    fn DNSServiceRefDeallocate(sd_ref: DNSServiceRef);
}

#[derive(Clone, Debug)]
pub enum ServiceError {
    NoError,
    Unknown,
    NoSuchName,
    NoMemory,
    BadParam,
    BadReference,
    BadState,
    BadFlags,
    Unsupported,
    NotInitialized,
    AlreadyRegistered,
    NameConflict,
    Invalid,
    Firewall,
    Incompatible,
    BadInterfaceIndex,
    Refused,
    NoSuchRecord,
    NoAuth,
    NoSuchKey,
    NATTraversal,
    DoubleNAT,
    BadTime,
    BadSig,
    BadKey,
    Transiet,
    ServiceNotRunning,
    NatPortMappingUnsupported,
    NatPortMappingDisabled,
    NoRouter,
    PollingMode,
    Timeout,
}

impl From<DNSServiceErrorType> for ServiceError {
    fn from(err: DNSServiceErrorType) -> Self {
        match err {
            -65537 => ServiceError::Unknown,
            -65538 => ServiceError::NoSuchName,
            -65539 => ServiceError::NoMemory,
            -65540 => ServiceError::BadParam,
            -65541 => ServiceError::BadReference,
            -65542 => ServiceError::BadState,
            -65543 => ServiceError::BadFlags,
            -65544 => ServiceError::Unsupported,
            -65545 => ServiceError::NotInitialized,
            -65547 => ServiceError::AlreadyRegistered,
            -65548 => ServiceError::NameConflict,
            -65549 => ServiceError::Invalid,
            -65550 => ServiceError::Firewall,
            -65551 => ServiceError::Incompatible,
            -65552 => ServiceError::BadInterfaceIndex,
            -65553 => ServiceError::Refused,
            -65554 => ServiceError::NoSuchRecord,
            -65555 => ServiceError::NoAuth,
            -65556 => ServiceError::NoSuchKey,
            -65557 => ServiceError::NATTraversal,
            -65558 => ServiceError::DoubleNAT,
            -65559 => ServiceError::BadTime,
            -65560 => ServiceError::BadSig,
            -65561 => ServiceError::BadKey,
            -65562 => ServiceError::Transiet,
            -65563 => ServiceError::ServiceNotRunning,
            -65564 => ServiceError::NatPortMappingUnsupported,
            -65565 => ServiceError::NatPortMappingDisabled,
            -65566 => ServiceError::NoRouter,
            -65567 => ServiceError::PollingMode,
            -65568 => ServiceError::Timeout,
            _ => ServiceError::NoError,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ServiceError(ServiceError),
    IoError(io::Error),
}

impl From<DNSServiceErrorType> for Error {
    fn from(err: DNSServiceErrorType) -> Self {
        Error::ServiceError(ServiceError::from(err))
    }
}

impl From<ServiceError> for Error {
    fn from(err: ServiceError) -> Self {
        Error::ServiceError(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Service {
    pub name: String,
    pub regtype: String,
    pub domain: String,
}

#[derive(Clone, Debug)]
pub struct Host {
    pub name: String,
    pub port: u16,
}

#[derive(Clone, Debug)]
pub enum BrowseEvent {
    Joined(Service),
    Dropped(Service),
}

#[derive(Debug)]
pub struct Registration {
    sd_ref: BoxedDNSServiceRef,
    service: Service,
}

impl Default for Service {
    fn default() -> Self {
        Service {
            name: String::new(),
            regtype: String::new(),
            domain: String::new(),
        }
    }
}

#[derive(Debug)]
struct Socket {
    raw_fd: dnssd_sock_t,
}

impl mio::Evented for Socket {
    fn register(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.raw_fd).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.raw_fd).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        EventedFd(&self.raw_fd).deregister(poll)
    }
}

pub fn register_service() -> Result<impl Future<Item = Registration, Error = Error>, Error> {
    let service_result_mutex: &'static mut Mutex<Result<Service, ServiceError>> =
        Box::leak(Box::new(Mutex::new(Ok(Service::default()))));
    let sd_ref = dns_service_register(service_result_mutex)?;
    let raw_fd = dns_service_ref_socket(&sd_ref)?;
    Ok(wait_for_socket(raw_fd).then(move |result| {
        result?;
        dns_service_process_result(&sd_ref)?;
        service_result_mutex
            .lock()
            .unwrap()
            .clone()
            .map(|service| Registration { sd_ref, service })
            .map_err(|e| Error::from(e))
    }))
}

pub fn browse_services() -> Result<impl Stream<Item = BrowseEvent, Error = Error>, Error> {
    let browse_event: &'static mut Mutex<Result<BrowseEvent, ServiceError>> = Box::leak(Box::new(
        Mutex::new(Ok(BrowseEvent::Joined(Service::default()))),
    ));
    let sd_ref = dns_service_browse(browse_event)?;
    let raw_fd = dns_service_ref_socket(&sd_ref)?;
    Ok(socket_ready_stream(raw_fd).then(move |result| {
        result?;
        dns_service_process_result(&sd_ref)?;
        browse_event
            .lock()
            .unwrap()
            .clone()
            .map_err(|e| Error::from(e))
    }))
}

pub fn resolve_service(
    service: &Service,
) -> Result<impl Future<Item = Host, Error = Error>, Error> {
    let host_result_mutex: &'static mut Mutex<Result<Host, ServiceError>> =
        Box::leak(Box::new(Mutex::new(Ok(Host {
            name: String::new(),
            port: 0,
        }))));
    let sd_ref = dns_service_resolve(service, host_result_mutex)?;
    let raw_fd = dns_service_ref_socket(&sd_ref)?;
    Ok(wait_for_socket(raw_fd).then(move |result| {
        result?;
        dns_service_process_result(&sd_ref)?;
        host_result_mutex
            .lock()
            .unwrap()
            .clone()
            .map_err(|e| Error::from(e))
    }))
}

pub fn get_address(host: &Host) -> Result<impl Future<Item = IpAddr, Error = Error>, Error> {
    let ipaddr_mutex: &'static mut Mutex<Result<IpAddr, ServiceError>> = Box::leak(Box::new(
        Mutex::new(Ok(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))),
    ));
    let sd_ref = dns_service_get_addr_info(host, ipaddr_mutex)?;
    let raw_fd = dns_service_ref_socket(&sd_ref)?;
    Ok(wait_for_socket(raw_fd).then(move |result| {
        result?;
        dns_service_process_result(&sd_ref)?;
        ipaddr_mutex
            .lock()
            .unwrap()
            .clone()
            .map_err(|e| Error::from(e))
    }))
}

pub fn wait_for_socket(raw_fd: dnssd_sock_t) -> SocketReadyFuture {
    SocketReadyFuture {
        socket: PollEvented2::new(Socket { raw_fd }),
    }
}

pub struct SocketReadyFuture {
    socket: PollEvented2<Socket>,
}

impl Future for SocketReadyFuture {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let result = try_ready!(self.socket.poll_read_ready(mio::Ready::readable()));
        if result.is_readable() {
            Ok(Async::Ready(()))
        } else {
            Ok(Async::NotReady)
        }
    }
}

pub fn socket_ready_stream(raw_fd: dnssd_sock_t) -> SocketReadyStream {
    SocketReadyStream {
        socket: PollEvented2::new(Socket { raw_fd }),
    }
}

pub struct SocketReadyStream {
    socket: PollEvented2<Socket>,
}

impl Stream for SocketReadyStream {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let result = try_ready!(self.socket.poll_read_ready(mio::Ready::readable()));
        if result.is_readable() {
            Ok(Async::Ready(Some(())))
        } else {
            Ok(Async::NotReady)
        }
    }
}
