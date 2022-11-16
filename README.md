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

## A brief overview of TCP/IP 

'TCP/IP' can mean anything to do with the TCP and IP protocols -- applications (e.g. TELNET, FTP, rcp, etc.), network mediums, even other protocols (e.g. UDP, ARP, ICMP, etc.).

### Basic Structure 

Ethernet frames are the lowest level network unit.
They can be ARP (Address Resolution Protocol), or IP (Internet Protocol).
IP packets can be TCP, or UDP.

Data on an Ethernet is called an Ethernet Frame.
Data between the Ethernet driver and IP module is an IP packet.
Data between the IP module and the UDP module is a UDP datagram.
Data between the IP module and the TCP module is a TCP segment (or transport message).
Data in a network application is an application message.

Software that communicates with network interface hardware is a driver.
Software that communicates either with a driver, with network applications, or with other modules is a module.

Data flows towards the ethernet layer dependant on its used protocols.
For example, FTP is an application that used TCP.
The FTP protocol stack is FTP -> TCP -> IP -> ENET 

If a unit of data is passed to the ethernet driver, then to the IP module, then to the TCP module, the application message is passed upwards to the network application based on the value of the port field in the TCP header.

Upon receiving a unit of network data, a computer may sent that data back out onto another network.
This is called forwarding.
A computer whose dedicated role is forwarding IP packets is an IP-router.

#### Some questions we hope to answer:

* When we send an IP packet, how is the destination ethernet address determined ?

* How does IP know which of multiple lower network interfaces to use when sending out an IP packet ?

* How does a client on one computer reach the server on another ?

* Why do both TCP and UDP exist ?

* What network applications are avaliable ?

### Ethernet

An ethernet frame contains:
* Destination address
* Source address
* Type field 
* Data

An ethernet address is 6 bytes long.
Each device has its own ethernet address.

An ethernet frame with a destination "FF-FF-FF-FF-FF-FF" is a broadcast -- this address is used as a wildcard.

### ARP

ARP (Address Resolution Protocol) translates IP addresses to ethernet addresses.
The translation is done using a lookup table.

Applications sending data using IP will send data to another IP address.
That means that on its way, ARP will use that destination IP address to find the destination ethernet address so the data can be sent.

### Internet Protocol

At the core of IP is its route table.
The contents of the route table is defined by the network administrator.

#### Direct Routing

3 computers, A, B, C.
Each has one ethernet interface with an ethernet address, and one IP address assigned by the network manager.
The ethernet has an IP network number.

A sends an IP packet to B.
The packet contains an IP header, with A's IP address listed as the source IP address, and the ethernet frame header contains A's ethernet address listed as the source ethernet address.
It also contains B's IP address as the destination IP address, and the ethernet frame header contains B's ethernet address as the destination ethernet address.

#### Indirect Routing 

3 ethernets & IP networks, linked by an IP-router, called computer D.
Each IP network has 4 computers, each with an IP address and ethernet address.

Computer D has 3 IP addresses and 3 ethernet addresses -- one for each network it is connected to.

A sends an IP packet to B.
If A and B are on the same IP network, the process for communication is identical to the direct method.

A sends an IP packet to D.
D is the IP-router that connects the three IP networks.
Again, this is a direct communication.

A sents an IP packet to E, where A and E are on different IP networks.
The communication is no longer direct.
A now must use D to forward the IP packet to the next IP network.
This is indirect communication.

Again, the data that A sends contains A's IP and ethernet addresses, and E's IP address. 
This time, however, the ethernet address it contains as destination is D's.
This is how D will know that the data is meant to be passed along, as the destination IP and ethernet addresses will not yet correspond.

#### IP Module Routing Rules

When an IP packet is sent, it must determine whether it is to be sent directly or indirectly.
This is decided by consulting the route table.

When an IP packet is received, it must either be forwareded, or passed upwards.
If forwarded, it is treated as an outgoing packet.

#### IP Address

An IP address is 4 bytes, set by the network manager.
One part is the IP network number.
The other part is the IP computer/host number.

There are conventions to determine how many bits are dedicated to each part, and to how the network numbers are distributed to avoid clashes.

#### Names

Names can be used as aliases for addresses.
