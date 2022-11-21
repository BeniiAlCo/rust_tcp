use crate::tcp::TcpState;
use std::collections::HashMap;
use std::io;
use std::net::Ipv4Addr;

mod network_parse;
mod tcp;

type Port = u16;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
struct Quad {
    source: (Ipv4Addr, Port),
    destination: (Ipv4Addr, Port),
}

fn main() -> io::Result<()> {
    let mut connections: HashMap<Quad, TcpState> = Default::default();

    let mut nic = tun_tap::Iface::without_packet_info("tun0", tun_tap::Mode::Tun)?;
    let mut buf = vec![0u8; 1504];

    loop {
        let nbytes = nic.recv(&mut buf[..])?;
        // if we get the tun interface with packet info, need to strip leading 4 bits, and then add
        // them pack on when sending a packet
        //
        //let _flags = u16::from_be_bytes([buf[0], buf[1]]);
        //let proto = u16::from_be_bytes([buf[2], buf[3]]);
        //
        // // no non-ipv4
        //if proto != 0x0800 {
        //    continue;
        //}

        match etherparse::Ipv4HeaderSlice::from_slice(&buf[..nbytes]) {
            Ok(ip_header) => {
                // (source_ip, source_port, destination_ip, destination_port)
                // This is a single connection in the TCP/IP protocol
                // When we use TCP/IP, we will generate a map from this quad, to the state for the
                // connection it represents

                let _bytes = nbytes - 4;
                let ip_source = ip_header.source_addr();
                let ip_destination = ip_header.destination_addr();
                let protocol = ip_header.protocol();

                //let payload_length = packet.payload_len();
                //eprintln!("read {bytes} bytes (flags: {flags}, proto: {proto}): {packet:?}");

                if protocol != 0x06 {
                    // no non-tcp packets
                    continue;
                }

                match etherparse::TcpHeaderSlice::from_slice(&buf[ip_header.slice().len()..nbytes])
                {
                    Ok(tcp_header) => {
                        // Once here, we know we have recieved a tcp packet.
                        // From here, we want to check to see if we have receieved data from this
                        // address before (and if so, continue from the current state in the tcp
                        // handshake process with that address), or add it as a new connection (and
                        // thus start the tcp handshake process)
                        //
                        // In the future, it is at this part of the process that we can also think
                        // about filtering out different kinds of connections, i.e. connections to
                        // specific ports

                        use std::collections::hash_map::Entry;

                        let ip_header_size = ip_header.slice().len();
                        let tcp_header_size = tcp_header.slice().len();
                        let data_start_index = ip_header_size + tcp_header_size;
                        let source_port = tcp_header.source_port();
                        let destination_port = tcp_header.destination_port();
                        let payload = &buf[data_start_index..nbytes];

                        match connections.entry(Quad {
                            source: (ip_source, source_port),
                            destination: (ip_destination, destination_port),
                        }) {
                            Entry::Occupied(mut c) => {
                                c.get_mut()
                                    .on_packet(&mut nic, ip_header, tcp_header, payload)?;
                            }
                            Entry::Vacant(e) => {
                                if let Some(c) =
                                    tcp::TcpState::accept(&mut nic, ip_header, tcp_header, payload)?
                                {
                                    e.insert(c);
                                }
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("ignoring weird packet {err:?}");
                    }
                }
            }
            Err(err) => {
                eprintln!("ignoring weird packet {err:?}");
            }
        }
    }
}
