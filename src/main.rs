use std::io;

fn main() -> io::Result<()> {
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

                match etherparse::TcpHeaderSlice::from_slice(&buf[4 + packet.slice().len()..]) {
                    Ok(packet) => {
                        let port = packet.destination_port();
                        let payload_length = packet.slice().len();

                        eprintln!(
                            "{source} -> {destination} {payload_length}b of tcp to port {port}"
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
