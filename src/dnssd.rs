use libc::{c_char, c_int, c_void, int32_t, uint16_t, uint32_t};
use mio;
use mio::unix::EventedFd;
use std::convert::From;
use std::ffi::{CStr, CString};
use std::io;
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

#[allow(non_camel_case_types)]
pub type dnssd_sock_t = c_int;

pub type DNSServiceErrorType = int32_t;

pub const DNSSERVICEERR_NOERROR: DNSServiceErrorType = 0;
pub const DNSSERVICEERR_UNKNOWN: DNSServiceErrorType = -65537; /* 0xFFFE FFFF */
pub const DNSSERVICEERR_NOSUCHNAME: DNSServiceErrorType = -65538;
pub const DNSSERVICEERR_NOMEMORY: DNSServiceErrorType = -65539;
pub const DNSSERVICEERR_BADPARAM: DNSServiceErrorType = -65540;
pub const DNSSERVICEERR_BADREFERENCE: DNSServiceErrorType = -65541;
pub const DNSSERVICEERR_BADSTATE: DNSServiceErrorType = -65542;
pub const DNSSERVICEERR_BADFLAGS: DNSServiceErrorType = -65543;
pub const DNSSERVICEERR_UNSUPPORTED: DNSServiceErrorType = -65544;
pub const DNSSERVICEERR_NOTINITIALIZED: DNSServiceErrorType = -65545;
pub const DNSSERVICEERR_ALREADYREGISTERED: DNSServiceErrorType = -65547;
pub const DNSSERVICEERR_NAMECONFLICT: DNSServiceErrorType = -65548;
pub const DNSSERVICEERR_INVALID: DNSServiceErrorType = -65549;
pub const DNSSERVICEERR_FIREWALL: DNSServiceErrorType = -65550;
pub const DNSSERVICEERR_INCOMPATIBLE: DNSServiceErrorType = -65551; /* client library incompatible with daemon */
pub const DNSSERVICEERR_BADINTERFACEINDEX: DNSServiceErrorType = -65552;
pub const DNSSERVICEERR_REFUSED: DNSServiceErrorType = -65553;
pub const DNSSERVICEERR_NOSUCHRECORD: DNSServiceErrorType = -65554;
pub const DNSSERVICEERR_NOAUTH: DNSServiceErrorType = -65555;
pub const DNSSERVICEERR_NOSUCHKEY: DNSServiceErrorType = -65556;
pub const DNSSERVICEERR_NATTRAVERSAL: DNSServiceErrorType = -65557;
pub const DNSSERVICEERR_DOUBLENAT: DNSServiceErrorType = -65558;
pub const DNSSERVICEERR_BADTIME: DNSServiceErrorType = -65559; /* Codes up to here existed in Tiger */
pub const DNSSERVICEERR_BADSIG: DNSServiceErrorType = -65560;
pub const DNSSERVICEERR_BADKEY: DNSServiceErrorType = -65561;
pub const DNSSERVICEERR_TRANSIENT: DNSServiceErrorType = -65562;
pub const DNSSERVICEERR_SERVICENOTRUNNING: DNSServiceErrorType = -65563; /* Background daemon not running */
pub const DNSSERVICEERR_NATPORTMAPPINGUNSUPPORTED: DNSServiceErrorType = -65564; /* NAT doesn't support PCP, NAT-PMP or UPnP */
pub const DNSSERVICEERR_NATPORTMAPPINGDISABLED: DNSServiceErrorType = -65565; /* NAT supports PCP, NAT-PMP or UPnP, but it's disabled by the administrator */
pub const DNSSERVICEERR_NOROUTER: DNSServiceErrorType = -65566; /* No router currently configured (probably no network connectivity) */
pub const DNSSERVICEERR_POLLINGMODE: DNSServiceErrorType = -65567;
pub const DNSSERVICEERR_TIMEOUT: DNSServiceErrorType = -65568;

#[allow(unused)]
type DNSServiceRegisterReply = extern "C" fn(
    sd_ref: DNSServiceRef,
    flags: DNSServiceFlags,
    error_code: int32_t,
    name: *const c_char,
    regtype: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
);

#[allow(unused)]
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

extern "C" fn dns_service_register_cb(
    _sd_ref: DNSServiceRef,
    _flags: uint32_t,
    _error_code: DNSServiceErrorType,
    name: *const c_char,
    regtype: *const c_char,
    domain: *const c_char,
    context: *mut c_void,
) {
    let service_mutex: &mut Mutex<Service> = unsafe { &mut *(context as *mut Mutex<Service>) };
    let mut service_guard = service_mutex.lock().unwrap();
    unsafe {
        (*service_guard).name = CStr::from_ptr(name).to_string_lossy().into_owned();
        (*service_guard).regtype = CStr::from_ptr(regtype).to_string_lossy().into_owned();
        (*service_guard).domain = CStr::from_ptr(domain).to_string_lossy().into_owned();
    };
}

pub fn dns_service_register(
    service_mutex: &mut Mutex<Service>,
) -> Result<BoxedDNSServiceRef, DNSServiceErrorType> {
    let reg_type = CString::new("_localchat._tcp.").unwrap();
    let context = service_mutex as *mut _ as *mut c_void;
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
        if err == DNSSERVICEERR_NOERROR {
            Ok(BoxedDNSServiceRef(sd_ref))
        } else {
            Err(err)
        }
    }
}

extern "C" fn dns_service_browse_cb(
    _sd_ref: DNSServiceRef,
    _flags: DNSServiceFlags,
    _interface_index: uint32_t,
    _error_code: DNSServiceErrorType,
    _service_name: *const c_char,
    _regtype: *const c_char,
    _reply_domain: *const c_char,
    _context: *mut c_void,
) {
    println!("dns service browse callback got called!");
}

pub fn dns_service_browse() -> DNSServiceErrorType {
    unsafe {
        let mut sd_ref: DNSServiceRef = ptr::null_mut();
        let sd_ref_ptr = &mut sd_ref as *mut DNSServiceRef;
        let reg_type = CString::new("_localchat._tcp.").unwrap();
        DNSServiceBrowse(
            sd_ref_ptr,
            0,
            0,
            reg_type.as_ptr(),
            ptr::null(),
            dns_service_browse_cb,
            ptr::null_mut(),
        )
    }
}

pub fn dns_service_ref_socket(
    boxed_sd_ref: &BoxedDNSServiceRef,
) -> Result<dnssd_sock_t, DNSServiceErrorType> {
    let sock_fd = unsafe { DNSServiceRefSockFD(boxed_sd_ref.0) };
    if sock_fd == -1 {
        Err(DNSSERVICEERR_UNKNOWN)
    } else {
        Ok(sock_fd)
    }
}

pub fn dns_service_process_result(boxed_sd_ref: &BoxedDNSServiceRef) -> DNSServiceErrorType {
    unsafe { DNSServiceProcessResult(boxed_sd_ref.0) }
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

    fn DNSServiceRefSockFD(sd_ref: DNSServiceRef) -> dnssd_sock_t;

    fn DNSServiceProcessResult(sd_ref: DNSServiceRef) -> DNSServiceErrorType;

    fn DNSServiceRefDeallocate(sd_ref: DNSServiceRef);
}

#[derive(Debug)]
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

#[derive(Clone, Debug)]
pub struct Service {
    name: String,
    regtype: String,
    domain: String,
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

pub fn register_service() -> Result<impl Future<Item = Service, Error = Error>, Error> {
    let service_mutex: &'static mut Mutex<Service> =
        Box::leak(Box::new(Mutex::new(Service::default())));
    let boxed_sd_ref = dns_service_register(service_mutex)?;
    let raw_fd = dns_service_ref_socket(&boxed_sd_ref)?;
    Ok(wait_for_socket(raw_fd).map(move |_| {
        // Will synchronously trigger our "callback"
        dns_service_process_result(&boxed_sd_ref);
        let service = service_mutex.lock().unwrap();
        (*service).clone()
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
