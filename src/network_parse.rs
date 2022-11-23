use nom::{
    bits::{bits, streaming::take},
    error::Error,
    sequence::tuple,
    IResult,
};
use std::net::Ipv4Addr;

#[derive(Debug, Clone, Copy)]
pub struct TunTapHeader {
    pub flags: Flags,
    pub protocol: Option<Protocol>,
}

#[derive(Debug, Clone, Copy)]
pub enum Flags {
    NoFlagsSet,
    IffNoPi,
    IffTun,
    IffTap,
}

#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    Ipv4,
    Other,
}

impl TunTapHeader {
    pub fn from_slice(slice: &[u8]) -> Result<TunTapHeader, Box<dyn std::error::Error>> {
        // Is the slice big enough to hold the header?
        assert!(slice.len() >= 2);

        // Flags
        let (input, flags) = parse_flags(slice).unwrap();

        // Protocol
        // If the 'No Packet Information' flag is set, skip this !
        let protocol = if let Flags::IffNoPi = flags {
            None
        } else {
            let (_, protocol) = parse_protocol(input).unwrap();
            Some(protocol)
        };
        // Seems fine!
        Ok(TunTapHeader { flags, protocol })
    }

    pub fn flags(&self) -> Flags {
        self.flags
    }

    pub fn protocol(&self) -> Option<Protocol> {
        self.protocol
    }

    pub fn header_len(&self) -> usize {
        if let Flags::IffNoPi = self.flags {
            2
        } else {
            4
        }
    }
}
fn parse_flags(input: &[u8]) -> IResult<&[u8], Flags> {
    let (input, flags) = take_16b(input).unwrap();

    // IFF_NO_PI = 0x1000
    // IFF_TUN = 0x0001
    // IFF_TAP = 0x0002
    let flags = match flags {
        0x0000 => Flags::NoFlagsSet,
        0x1000 => Flags::IffNoPi,
        0x0001 => Flags::IffTun,
        0x0002 => Flags::IffTap,
        _ => {
            panic!("Something went wrong with the tun/tap header flags");
        }
    };

    Ok((input, flags))
}

fn parse_protocol(input: &[u8]) -> IResult<&[u8], Protocol> {
    let (input, protocol) = take_16b(input).unwrap();

    // IPv4 = 0x0800
    let protocol = match protocol {
        0x0800 => Protocol::Ipv4,
        _ => Protocol::Other,
    };

    Ok((input, protocol))
}

#[derive(Debug)]
pub struct IPv4Header {
    version: u8,
    ihl: u8,
    type_of_service: u8,
    total_length: u16,
    identification: u16,
    flags: u8,
    fragment_offset: u16,
    time_to_live: u8,
    protocol: u8,
    header_checksum: u16,
    source_address: Ipv4Addr,
    destination_address: Ipv4Addr,
    //options: Option<u32>,
    //padding: Option<u32>,
}

pub fn parse_ipv4(input: &[u8]) -> IResult<&[u8], IPv4Header> {
    let version = take(4usize);
    let ihl = take(4usize);
    let type_of_service = take(8usize);
    let total_length = take(16usize);
    let identification = take(16usize);
    let flags = take(3usize);
    let fragment_offset = take(13usize);
    let time_to_live = take(8usize);
    let protocol = take(8usize);
    let header_checksum = take(16usize);
    let source_address = take(32usize);
    let destination_address = take(32usize);
    //let options = ;
    //let padding =;

    let parser = tuple((
        version,
        ihl,
        type_of_service,
        total_length,
        identification,
        flags,
        fragment_offset,
        time_to_live,
        protocol,
        header_checksum,
        source_address,
        destination_address,
    ));

    let (
        input,
        (
            version,
            ihl,
            type_of_service,
            total_length,
            identification,
            flags,
            fragment_offset,
            time_to_live,
            protocol,
            header_checksum,
            source_address,
            destination_address,
        ),
    ): (_, (_, _, _, _, _, _, _, _, _, _, u32, u32)) =
        bits::<&[u8], _, Error<(&[u8], usize)>, Error<&[u8]>, _>(parser)(input)
            .expect("Input should contain atleast 20 bytes (the IPv4 header)");

    let source_address = Ipv4Addr::from(source_address);
    let destination_address = Ipv4Addr::from(destination_address);

    Ok((
        input,
        IPv4Header {
            version,
            ihl,
            type_of_service,
            total_length,
            identification,
            flags,
            fragment_offset,
            time_to_live,
            protocol,
            header_checksum,
            source_address,
            destination_address,
        },
    ))
}

fn take_16b(input: &[u8]) -> IResult<&[u8], u16> {
    bits::<&[u8], u16, Error<(&[u8], usize)>, Error<&[u8]>, _>(take(16usize))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tun_tap_parser() {
        let input: &[u8] = &[0, 0, 8, 0, 2, 3];
        let header = TunTapHeader::from_slice(input).unwrap();
        assert!(matches!(header.flags, Flags::NoFlagsSet));
        assert!(matches!(header.protocol, Some(Protocol::Ipv4)));
    }

    #[test]
    #[should_panic]
    fn tun_tap_parser_missing_arguments() {
        let input: &[u8] = &[0];
        TunTapHeader::from_slice(input).unwrap();
    }

    #[test]
    fn placeholder_test() {
        let input: &[u8] = &[
            69, 0, 0, 84, 71, 99, 64, 0, 64, 1, 113, 242, 192, 168, 0, 1, 192, 168, 0, 2, 8, 0, 76,
            178, 0, 24, 0, 1, 67, 191, 123, 99, 0, 0, 0, 0, 38, 63, 7, 0, 0, 0, 0, 16, 17, 18, 19,
            20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41,
            42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55,
        ];
        let output = parse_ipv4(input).unwrap();
        let remaining = output.0;
        let header = output.1;

        assert_eq!(
            remaining,
            &[
                8, 0, 76, 178, 0, 24, 0, 1, 67, 191, 123, 99, 0, 0, 0, 0, 38, 63, 7, 0, 0, 0, 0,
                16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36,
                37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55
            ]
        );
        assert_eq!(header.version, 0b0100); // 4
        assert_eq!(header.ihl, 0b0101); // 5
        assert_eq!(header.type_of_service, 0);
        assert_eq!(header.total_length, 84);
        assert_eq!(header.identification, 18275);
        assert_eq!(header.flags, 0b010); // 2
        assert_eq!(header.fragment_offset, 0);
        assert_eq!(header.time_to_live, 64);
        assert_eq!(header.protocol, 1);
        assert_eq!(header.header_checksum, 29170);
        assert_eq!(header.source_address, Ipv4Addr::new(192, 168, 0, 1));
        assert_eq!(header.destination_address, Ipv4Addr::new(192, 168, 0, 2));
    }
}
