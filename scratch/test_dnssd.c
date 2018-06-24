#include <dns_sd.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/select.h>
#include <unistd.h>

void registered_cb(DNSServiceRef sdRef, DNSServiceFlags flags,
                        DNSServiceErrorType errorCode, const char *name,
                        const char *regtype, const char *domain,
                        void *context) {
  if (errorCode == kDNSServiceErr_NoError) {
    printf("Successfully registered service: %s with regtype: %s\n", name, regtype);
  } else {
    fprintf(stderr, "Could not register service\n");
  }
}

void test_register() {
  DNSServiceRef sdRef;
  char *regType = "_localchat._tcp.";
  uint16_t port = 1337;
  DNSServiceErrorType err;

  err = DNSServiceRegister(&sdRef, 0, 0, NULL, regType, NULL, NULL, port, 0,
                           NULL, &registered_cb, NULL);

  if (err != kDNSServiceErr_NoError) {
    fprintf(stderr, "Failed to register service\n");
    exit(1);
  } else {
    printf("Successfully registered service\n");
  }
}

void browse_service_cb(
    DNSServiceRef sdRef,
    DNSServiceFlags flags,
    uint32_t interfaceIndex,
    DNSServiceErrorType errorCode,
    const char                          *serviceName,
    const char                          *regtype,
    const char                          *replyDomain,
    void                                *context) {
  if (flags & kDNSServiceFlagsAdd) {
    printf("Service added: '%s.%s%s'\n", serviceName, regtype, replyDomain);
  } else {
    printf("Service removed: '%s.%s%s'\n", serviceName, regtype, replyDomain);
  }
}

void test_browse_service(DNSServiceRef *sdRef) {
  DNSServiceErrorType err;
  const char *regType = "_localchat._tcp";
  err = DNSServiceBrowse(sdRef,
                   0,
                   0,
                   regType,
                   NULL,    /* may be NULL */
                   browse_service_cb,
                   NULL);
  if (err != kDNSServiceErr_NoError) {
    fprintf(stderr, "Failed to call DNSServiceBrowse!\n");
    exit(1);
  }
}

int main() {
  DNSServiceRef sdRef;

  test_browse_service(&sdRef);

  // get the socket descriptor that the client/daemon use to communicate
  dnssd_sock_t sockfd = DNSServiceRefSockFD(sdRef);

  // run a select loop, sleeping until we can call a callback
  fd_set fds;
  FD_ZERO(&fds);        // initialize an fd set for select
  FD_SET(sockfd, &fds); // add the socket fd to the fd set
  DNSServiceErrorType err;
  for (;;) {
    int num_ready = select(sockfd + 1, &fds, NULL, NULL, NULL);
    if (!num_ready) { continue; };
    // we can read from the socket and trigger our callbacks
    err = DNSServiceProcessResult(sdRef);
    if (err != kDNSServiceErr_NoError) {
      // An error occurred
      fprintf(stderr, "Processing a DNS service result failed with error: %d\n", err);
    }
  }
}
