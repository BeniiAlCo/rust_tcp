use std::io;

enum ConnectionState {
    //Closed,
    //Listen,
    SynRcvd,
    Estab,
}

pub struct TcpState {
    connection_state: ConnectionState,
    send: SendSequence,
    recieve: RecieveSequence,
    ip: etherparse::Ipv4Header,
}

// the send sequence space is the list of positions of the data we have sent
// una is the newest point to be unacknowledged -- everything before it has been acknowledged
// nxt is the next data point to be sent -- everything between una and nxt has been sent but not
// acknowledged
// wnd is the size of data that is sent at one time, so between una and una+wnd, we can keep
// sending more data, but once we reach that, we must stop and wait for a response
// everything after una+wnd cannot be sent yet
#[derive(Copy, Clone, Debug)]
pub struct SendSequence {
    una: u32,
    nxt: u32,
    wnd: u16,
    up: bool,
    wl1: usize,
    wl2: usize,
    iss: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct RecieveSequence {
    nxt: u32,
    wnd: u16,
    up: bool,
    irs: u32,
}

impl TcpState {
    pub fn accept<'a>(
        //self,
        nic: &mut tun_tap::Iface,
        ip_header: etherparse::Ipv4HeaderSlice,
        tcp_header: etherparse::TcpHeaderSlice,
        data: &'a [u8],
    ) -> io::Result<Option<Self>> {
        let mut buf = [0u8; 1500];

        let _source_address = ip_header.source_addr();
        let source_port = tcp_header.source_port();
        let _destination_address = ip_header.destination_addr();
        let destination_port = tcp_header.destination_port();
        let _payload_size = data.len();

        if !tcp_header.syn() {
            // got unexpected non-SYN packet
            Result::Ok(None)
        } else {
            // we have received a SYN packet, and we can start to establish a connection by
            // returning a SYN,ACK packet

            let iss = 0;
            let mut connection = TcpState {
                connection_state: ConnectionState::SynRcvd,
                recieve: RecieveSequence {
                    nxt: tcp_header.sequence_number() + 1,
                    irs: tcp_header.sequence_number(),
                    wnd: tcp_header.window_size(),
                    up: false,
                },
                send: SendSequence {
                    iss,
                    una: iss,
                    nxt: iss + 1,
                    wnd: 8,
                    up: false,
                    wl1: 0,
                    wl2: 0,
                },
                ip: etherparse::Ipv4Header::new(
                    0,
                    64,
                    etherparse::ip_number::TCP,
                    [
                        ip_header.destination()[0],
                        ip_header.destination()[1],
                        ip_header.destination()[2],
                        ip_header.destination()[3],
                    ],
                    [
                        ip_header.source()[0],
                        ip_header.source()[1],
                        ip_header.source()[2],
                        ip_header.source()[3],
                    ],
                ),
            };

            // construct the headers
            let outgoing_source_port = destination_port;
            let outgoing_destination_port = source_port;
            let _outgoing_sequence_number = 0;
            let _outgoing_window_size = 8;

            // keep track of sender info

            // establish what it is we will be sending back

            // need to start establishing a connection
            let mut syn_ack = etherparse::TcpHeader::new(
                outgoing_source_port,
                outgoing_destination_port,
                connection.send.iss,
                connection.send.wnd,
            );
            syn_ack.acknowledgment_number = connection.recieve.nxt;
            syn_ack.syn = true;
            syn_ack.ack = true;
            let bytes_length = 0;
            connection
                .ip
                .set_payload_len(syn_ack.header_len() as usize + bytes_length)
                .expect("payload length value is too big");

            // the kernel does the checksum for us !
            //syn_ack.checksum = syn_ack
            //   .calc_checksum_ipv4(&ip, &[])
            //    .expect("failed to compute checksum");

            // write the headers to a buffer, then send everything written, and exclude any
            // empty part of the buffer
            let unwritten = {
                let mut unwritten = &mut buf[..];
                connection.ip.write(&mut unwritten).ok();
                syn_ack.write(&mut unwritten).ok();
                unwritten.len()
            };
            //eprintln!("{:02x?}", &buf[..buf.len() - unwritten]);
            nic.send(&buf[..buf.len() - unwritten])?;
            Ok(Some(connection))
        }

        // eprintln!("{source_address}:{source_port} -> {destination_address}:{destination_port} {payload_size}b of tcp");
    }

    pub fn on_packet<'a>(
        &mut self,
        _nic: &mut tun_tap::Iface,
        _ip_header: etherparse::Ipv4HeaderSlice,
        tcp_header: etherparse::TcpHeaderSlice,
        data: &'a [u8],
    ) -> io::Result<()> {
        // acceptable ack check (RFC 793 S3.3)
        // SND.UNA < SEG.ACK =< SND.NXT (but it wraps !)
        let una = self.send.una;
        let ack = tcp_header.acknowledgment_number();
        let nxt = self.send.nxt;
        let mut slen = data.len();
        if tcp_header.fin() {
            slen += 1
        };
        if tcp_header.syn() {
            slen += 1
        };

        if !(una < ack && (ack <= nxt || (ack >= nxt && una > nxt)) || ack <= nxt && una > nxt) {
            return Ok(());
        };

        // valid segment check
        // RCV.NXT =< SEG.SEQ < RCV.NXT+RCV.WND
        let nxt = self.recieve.nxt;
        let seq = tcp_header.sequence_number();
        let end = self.recieve.nxt.wrapping_add(self.recieve.wnd as u32);
        let seq_end = tcp_header.sequence_number() + slen as u32 - 1;

        if slen == 0 {
            //zero-length segment rules
            if self.recieve.wnd == 0 {
                if seq != self.recieve.nxt {
                    return Ok(());
                }
            } else if !(nxt <= seq && (seq < end || (seq > end && nxt > end))
                || seq < end && nxt > end)
            {
                return Ok(());
            }
        } else if self.recieve.wnd == 0
            || !(nxt <= seq_end && (seq_end < end || (seq_end > end && nxt > end))
                || seq_end < end && nxt > end)
        {
            return Ok(());
        }

        match self.connection_state {
            //ConnectionState::Closed => todo!(),
            //ConnectionState::Listen => todo!(),
            ConnectionState::SynRcvd => {
                // if we're in this state, we're going to expect to receive an ACK for a SYN we
                // have previously sent

                if !tcp_header.ack() {
                    return Ok(());
                }

                self.connection_state = ConnectionState::Estab;

                // now we can terminate the connection !
            }
            ConnectionState::Estab => {
                unimplemented!()
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ack_acceptance() {
        let una = 0;
        let ack = 1;
        let nxt = 2;
        assert!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt);

        let una = 1;
        let ack = 2;
        let nxt = 0;
        assert!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt);

        let una = 2;
        let ack = 0;
        let nxt = 1;
        assert!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt);

        let una = 0;
        let ack = 1;
        let nxt = 1;
        assert!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt);

        let una = 1;
        let ack = 0;
        let nxt = 0;
        assert!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt);

        let una = 0;
        let ack = 2;
        let nxt = 1;
        assert!(!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt));

        let una = 2;
        let ack = 1;
        let nxt = 0;
        assert!(!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt));

        let una = 1;
        let ack = 0;
        let nxt = 2;
        assert!(!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt));

        let una = 0;
        let ack = 0;
        let nxt = 0;
        assert!(!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt));

        let una = 0;
        let ack = 1;
        let nxt = 0;
        assert!(!(una < ack && (ack <= nxt || una > nxt) || ack <= nxt && una > nxt));
    }
}
