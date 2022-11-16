# rust_tcp

## Overview

(Following the tutorial by Jon Gjengset, hosted at https://www.youtube.com/watch?v=bzja9fQWzdA)

TCP is one of the fundamental protocols of the internet.
It enables two hosts to talk to one another in a reliable way.
It puts in place certain guarentees about the data that is sent/recieved, such as the size, and order of the data.

The goal for this project is to implement something that can talk to a 'real server', for some definition of 'real' and 'server'.
That is, we want the ability to talk to some host that exists on the internet -- one that is not our own.

No advanced extensions;
No congestion control;
etc.

* This implementation will follow RFC 793: 

    https://www.rfc-editor.org/rfc/rfc793


* For a tutorial on the basics of TCP/IP, use: 

    https://www.rfc-editor.org/rfc/rfc1180


* RFC 7414 lists other relevant RFCs that may need to be implemented (here, the focus is on those listed under 'Core Functionality, in section 2 -- 793, 1122, 5681, 6093, 6298, 6691): 

    https://www.rfc-editor.org/rfc/rfc7414


* RFC 2525 lists a number of known implementation problems and there potential solutions: 

    https://www.rfc-editor.org/rfc/rfc2525


* RFC 2398 includes tools for testing an implementation:

    https://www.rfc-editor.org/rfc/rfc2398


* Finally, we need the RFC for the Internet protocol:

    https://www.rfc-editor.org/rfc/rfc791


## Background on relevant crates and features

### tuntap

    https://github.com/vorner/tuntap

The kernel already implements TCP/IP -- to implement it ourselves in the way that we are, we need to pre-empt this behaviour.
To do this, we will use TUN/TAP to create a userspace virtual network adapter.
That is, we will ask the kernel to construct a virtual network adapter for us, and we will use that to give us a 'clean' working environment that the kernel will not interfere with. Normally, a userspace application will make a network-related request, and the kernel will handle things from there. We, however, want to do this ourselves, so must create a space that will allow us to do this.

TUN/TAP will let us create a fake network interface that we can use to send information to the kernel, which will treat it as if it has come from a real network interface. Similarly, if the kernel attempt to use this fake NIC, it will send data as though it is real, but the data will be sent to our userspace program.

Because our implementation here will give us the raw bytes that the kernel sends to our NIC, and we want to recive IP addresses, we will also have to implement the Internet Protocol (thought this should be relatively straightforward !)

### etherparse

The focus of this project is the implementation of TCP/IP, not the parsing of network packets, so we can rely on already existing crates to do the parsing for us !
