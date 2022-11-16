use crate::tcp::TcpState;
use std::collections::HashMap;
use std::io;
use std::net::Ipv4Addr;

mod tcp;

type Port = u16;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
struct Quad {
    source: (Ipv4Addr, Port),
    destination: (Ipv4Addr, Port),
}

fn main() -> io::Result<()> {
    let mut connections: HashMap<Quad, TcpState> = Default::default();

    let nic = tun_tap::Iface::new("tun0", tun_tap::Mode::Tun)?;
    let mut buf = vec![0u8; 1504];

    loop {
        let nbytes = nic.recv(&mut buf[..])?;
        let _flags = u16::from_be_bytes([buf[0], buf[1]]);
        let proto = u16::from_be_bytes([buf[2], buf[3]]);

        // no non-ipv4
        if proto != 0x0800 {
            continue;
        }

        match etherparse::Ipv4HeaderSlice::from_slice(&buf[4..nbytes]) {
            Ok(packet) => {
                // (source_ip, source_port, destination_ip, destination_port)
                // This is a single connection in the TCP/IP protocol
                // When we use TCP/IP, we will generate a map from this quad, to the state for the
                // connection it represents

                let _bytes = nbytes - 4;
                let source = packet.source_addr();
                let destination = packet.destination_addr();
                let protocol = packet.protocol();

                //let payload_length = packet.payload_len();
                //eprintln!("read {bytes} bytes (flags: {flags}, proto: {proto}): {packet:?}");

                if protocol != 0x06 {
                    // no non-tcp packets
                    continue;
                }

                let ip_header_size = packet.slice().len();

                match etherparse::TcpHeaderSlice::from_slice(&buf[4 + packet.slice().len()..]) {
                    Ok(packet) => {
                        let source_port = packet.source_port();
                        let destination_port = packet.destination_port();
                        let payload_length = packet.slice().len();

                        connections
                            .entry(Quad {
                                source: (source, source_port),
                                destination: (destination, destination_port),
                            })
                            .or_default()
                            .on_packet(header);

                        eprintln!(
                            "{source} -> {destination} {payload_length}b of tcp to port {destination_port}"
                        );
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
