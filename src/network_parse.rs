use nom::{
    bits::{bits, streaming::take},
    error::Error,
    sequence::tuple,
    IResult,
};

pub struct TunTapHeader {
    flags: u8,
    proto: u8,
}

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
    source_address: u32,
    destination_address: u32,
    options: Option<u32>,
    padding: Option<u32>,
}

fn parse_tun_tap_header(input: &[u8]) -> IResult<&[u8], TunTapHeader> {
    let parser = tuple((take(8usize), take(8usize)));
    let (input, (flags, proto)) =
        bits::<&[u8], (u8, u8), Error<(&[u8], usize)>, Error<&[u8]>, _>(parser)(input)
            .expect("we take 2 bytes");
    Ok((input, TunTapHeader { flags, proto }))
}

fn parse_ipv4(input: &[u8]) -> IResult<&[u8], (u8, u8, u8)> {
    let version = take(4usize);
    let ihl = take(4usize);
    let type_of_service = take(8usize);

    let parser = tuple((version, ihl, type_of_service));

    let (input, (version, ihl, type_of_service)) =
        bits::<&[u8], (u8, u8, u8), Error<(&[u8], usize)>, Error<&[u8]>, _>(parser)(input)
            .expect("we take atleast 2 bytes");

    Ok((input, (version, ihl, type_of_service)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tun_tap_parser() {
        let input: &[u8] = &[0, 1, 2, 3];
        let output = parse_tun_tap_header(input).unwrap();
        let remaining = output.0;
        let header = output.1;
        assert_eq!(remaining, &[2, 3]);
        assert_eq!(header.flags, 0);
        assert_eq!(header.proto, 1);
    }

    #[test]
    #[should_panic]
    fn tun_tap_parser_missing_arguments() {
        let input: &[u8] = &[0];
        parse_tun_tap_header(input).unwrap();
    }

    #[test]
    fn placeholder_test() {
        let input: &[u8] = &[69, 0, 0, 84];
        let output = parse_ipv4(input).unwrap();
        let remaining = output.0;
        let header = output.1;
        let version = header.0;
        let ihl = header.1;
        let type_of_service = header.2;
        assert_eq!(remaining, &[0, 84]);
        assert_eq!(version, 4);
        assert_eq!(ihl, 5);
        assert_eq!(type_of_service, 0);
    }
}
