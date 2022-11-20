use std::io;

enum ConnectionState {
    //Closed,
    //Listen,
    SynRcvd,
    Estab,
    FinWait1,
    FinWait2,
    Closing,
    TimeWait,
}

impl ConnectionState {
    fn is_synchronized(&self) -> bool {
        match *self {
            ConnectionState::SynRcvd => false,
            ConnectionState::Estab => true,
            ConnectionState::FinWait1 => true,
            ConnectionState::FinWait2 => true,
            ConnectionState::Closing => true,
            ConnectionState::TimeWait => true,
        }
    }
}

pub struct TcpState {
    connection_state: ConnectionState,
    send: SendSequence,
    recieve: RecieveSequence,
    ip: etherparse::Ipv4Header,
    tcp: etherparse::TcpHeader,
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
            let wnd = 8;
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
                    nxt: iss,
                    wnd,
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
                tcp: etherparse::TcpHeader::new(destination_port, source_port, iss, wnd),
            };

            // keep track of sender info

            // establish what it is we will be sending back

            // need to start establishing a connection

            //connection.tcp.acknowledgment_number = connection.recieve.nxt;
            connection.tcp.syn = true;
            connection.tcp.ack = true;

            connection.write(nic, &[])?;

            Ok(Some(connection))
        }

        // eprintln!("{source_address}:{source_port} -> {destination_address}:{destination_port} {payload_size}b of tcp");
    }

    fn write(&mut self, nic: &mut tun_tap::Iface, payload: &[u8]) -> io::Result<usize> {
        let mut buf = [0u8; 1500];
        self.tcp.sequence_number = self.send.nxt;
        self.tcp.acknowledgment_number = self.recieve.nxt;

        let size = std::cmp::min(
            buf.len(),
            self.tcp.header_len() as usize + self.ip.header_len() + payload.len(),
        );
        self.ip.set_payload_len(size - self.ip.header_len());

        // the kernel does the checksum for us !
        self.tcp.checksum = self
            .tcp
            .calc_checksum_ipv4(&self.ip, &[])
            .expect("failed to compute checksum");

        // write the headers to a buffer, then send everything written, and exclude any
        // empty part of the buffer

        use std::io::Write;
        let mut unwritten = &mut buf[..];
        self.ip.write(&mut unwritten);
        self.tcp.write(&mut unwritten);
        let payload_bytes = unwritten.write(payload)?;
        let unwritten = unwritten.len();

        self.send.nxt = self.send.nxt.wrapping_add(payload_bytes as u32);
        if self.tcp.syn {
            self.send.nxt = self.send.nxt.wrapping_add(1);
            self.tcp.syn = false;
        }
        if self.tcp.fin {
            self.send.nxt = self.send.nxt.wrapping_add(1);
            self.tcp.fin = false;
        }

        //eprintln!("{:02x?}", &buf[..buf.len() - unwritten]);

        nic.send(&buf[..buf.len() - unwritten])?;
        Ok(payload_bytes)
    }

    pub fn snd_rst(&mut self, nic: &mut tun_tap::Iface) -> io::Result<()> {
        self.tcp.rst = true;
        self.tcp.sequence_number = 0;
        self.tcp.acknowledgment_number = 0;
        self.write(nic, &[])?;
        Ok(())
    }

    pub fn on_packet<'a>(
        &mut self,
        nic: &mut tun_tap::Iface,
        _ip_header: etherparse::Ipv4HeaderSlice,
        tcp_header: etherparse::TcpHeaderSlice,
        data: &'a [u8],
    ) -> io::Result<()> {
        // acceptable ack check (RFC 793 S3.3)
        // SND.UNA < SEG.ACK =< SND.NXT (but it wraps !)

        let mut slen = data.len();
        if tcp_header.fin() {
            slen += 1
        };
        if tcp_header.syn() {
            slen += 1
        };

        // valid segment check
        // RCV.NXT =< SEG.SEQ < RCV.NXT+RCV.WND
        let nxt = self.recieve.nxt;
        let seq = tcp_header.sequence_number();
        let end = self.recieve.nxt.wrapping_add(self.recieve.wnd as u32);
        let seq_end = tcp_header.sequence_number().wrapping_add(slen as u32 - 1);

        let okay = if slen == 0 {
            //zero-length segment rules
            if self.recieve.wnd == 0 {
                if seq != self.recieve.nxt {
                    false
                } else {
                    true
                }
            } else if !(nxt <= seq && (seq < end || (seq > end && nxt > end))
                || seq < end && nxt > end)
            {
                false
            } else {
                true
            }
        } else if self.recieve.wnd == 0
            || !(nxt <= seq_end && (seq_end < end || (seq_end > end && nxt > end))
                || seq_end < end && nxt > end)
        {
            false
        } else {
            true
        };

        if !okay {
            self.write(nic, &[])?;
            return Ok(());
        }
        self.recieve.nxt = tcp_header.sequence_number().wrapping_add(slen as u32);
        // TODO: we've gotta ACK this !

        if !tcp_header.ack() {
            return Ok(());
        }

        //self.tcp.fin = true;
        //self.write(nic, &[])?;
        //self.connection_state = ConnectionState::FinWait1;
        let una = self.send.una;
        let ack = tcp_header.acknowledgment_number();
        if let ConnectionState::SynRcvd = self.connection_state {
            if (una <= ack && (ack <= nxt || (ack >= nxt && una > nxt)) || ack <= nxt && una > nxt)
            {
                self.connection_state = ConnectionState::Estab;
            } else {
                // reset
            }
        }

        if let ConnectionState::Estab | ConnectionState::FinWait1 | ConnectionState::FinWait2 =
            self.connection_state
        {
            let nxt = self.send.nxt;

            if !(una < ack && (ack <= nxt || (ack >= nxt && una > nxt)) || ack <= nxt && una > nxt)
            {
                //if !self.connection_state.is_synchronized() {
                // send reset, as per spec
                // self.send.nxt = tcp_header.acknowledgment_number();
                //    self.snd_rst(nic)?;
                //}
                return Ok(());
            };

            self.send.una = tcp_header.acknowledgment_number();

            assert!(data.is_empty());

            if let ConnectionState::Estab = self.connection_state {
                self.tcp.fin = true;
                self.write(nic, &[])?;
                self.connection_state = ConnectionState::FinWait1;
            }
        }

        if let ConnectionState::FinWait1 = self.connection_state {
            if self.send.una == self.send.iss + 2 {
                // our fin has been ack'd

                // they must have ack'd our fin ! (it's all that we have sent)
                self.connection_state = ConnectionState::FinWait2;
            }
        }

        if tcp_header.fin() {
            match self.connection_state {
                ConnectionState::SynRcvd => unimplemented!(),
                ConnectionState::Estab => unimplemented!(),
                ConnectionState::FinWait1 => unimplemented!(),
                ConnectionState::FinWait2 => {
                    // we're done
                    self.write(nic, &[])?;
                    self.connection_state = ConnectionState::TimeWait;
                }
                ConnectionState::Closing => unimplemented!(),
                ConnectionState::TimeWait => unimplemented!(),
            }
        }

        // if let ConnectionState::FinWait2 = self.connection_state {
        //     if !tcp_header.fin() || !data.is_empty() {
        //         unimplemented!()
        //     }

        //     self.tcp.fin = false;
        //      self.write(nic, &[])?;
        //      self.connection_state = ConnectionState::Closing;
        //  }

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
