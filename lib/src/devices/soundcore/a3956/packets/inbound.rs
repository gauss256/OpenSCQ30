use nom::{
    IResult, Parser,
    bytes::complete::take,
    combinator::map,
    error::{ContextError, ParseError, context},
    multi::many0,
    number::complete::le_u8,
};

use crate::devices::soundcore::{
    a3956::structures::{A3956Info, TlvRecords},
    common::packet::{self, Command, inbound::FromPacketBody, outbound::ToPacket},
};

/// Unsolicited event sent by the earbuds when an Easy Chat session starts (body `00 01`)
/// or ends (body `00 00`). Observed on the Liberty 5 Pro; not known to be sent by other models.
///
/// The body is two bytes. The first is always `00` (likely a sub-command or reserved index);
/// the second is the active flag, so the flag is the last byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EasyChatEvent {
    pub is_active: bool,
}

impl EasyChatEvent {
    pub const COMMAND: Command = Command([0x31, 0x03]);
}

impl FromPacketBody for EasyChatEvent {
    type DirectionMarker = packet::InboundMarker;

    fn take<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
        input: &'a [u8],
    ) -> IResult<&'a [u8], Self, E> {
        context(
            "a3956 easy chat event",
            map((le_u8, le_u8), |(_reserved, flag)| Self {
                is_active: flag != 0,
            }),
        )
        .parse_complete(input)
    }
}

/// State update packet (command 0x0101) in the A3956 tag/length/value layout.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct A3956StateUpdatePacket {
    pub records: TlvRecords,
}

impl A3956StateUpdatePacket {
    /// Known tags. Everything else is kept raw in `records`.
    const TAG_BATTERY_LEFT: u8 = 0x03;
    const TAG_BATTERY_RIGHT: u8 = 0x04;
    const TAG_FIRMWARE_LEFT: u8 = 0x05;
    const TAG_FIRMWARE_RIGHT: u8 = 0x06;
    const TAG_SERIAL_NUMBER: u8 = 0x07;

    pub fn info(&self) -> A3956Info {
        fn ascii(bytes: Option<&[u8]>) -> String {
            bytes
                .map(|b| {
                    String::from_utf8_lossy(b)
                        .trim_end_matches('\0')
                        .to_owned()
                })
                .unwrap_or_default()
        }
        // Battery records are two bytes; the second is the percentage.
        fn battery(bytes: Option<&[u8]>) -> u8 {
            bytes.and_then(|b| b.last().copied()).unwrap_or_default()
        }
        A3956Info {
            serial_number: ascii(self.records.get(Self::TAG_SERIAL_NUMBER)),
            firmware_left: ascii(self.records.get(Self::TAG_FIRMWARE_LEFT)),
            firmware_right: ascii(self.records.get(Self::TAG_FIRMWARE_RIGHT)),
            battery_left: battery(self.records.get(Self::TAG_BATTERY_LEFT)),
            battery_right: battery(self.records.get(Self::TAG_BATTERY_RIGHT)),
        }
    }
}

fn take_tlv<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
    input: &'a [u8],
) -> IResult<&'a [u8], (u8, Vec<u8>), E> {
    let (input, tag) = le_u8(input)?;
    let (input, len) = le_u8(input)?;
    let (input, value) = take(len as usize)(input)?;
    Ok((input, (tag, value.to_vec())))
}

impl FromPacketBody for A3956StateUpdatePacket {
    type DirectionMarker = packet::InboundMarker;

    fn take<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
        input: &'a [u8],
    ) -> IResult<&'a [u8], Self, E> {
        context(
            "a3956 state update packet",
            map(many0(take_tlv), |records| Self {
                records: TlvRecords(records),
            }),
        )
        .parse_complete(input)
    }
}

impl ToPacket for A3956StateUpdatePacket {
    type DirectionMarker = packet::InboundMarker;

    fn command(&self) -> Command {
        packet::inbound::STATE_COMMAND
    }

    fn body(&self) -> Vec<u8> {
        self.records
            .0
            .iter()
            .flat_map(|(tag, value)| {
                std::iter::once(*tag)
                    .chain(std::iter::once(value.len() as u8))
                    .chain(value.iter().copied())
            })
            .collect()
    }
}

#[cfg(test)]
pub mod tests {
    use nom_language::error::VerboseError;

    use super::*;

    /// Captured from a Liberty 5 Pro on 2026-09-07 (body of the 0x0101 response, checksum stripped).
    pub const CAPTURED_STATE_BODY: &str = "010101010201010302005d0402005c050530352e3531060530352e35310711313230333030374631443232333431450008020054090530312e35340a01310b02fefe0c2000000000000000000000000000000000000000000000000000000000000000000d02060f0f02020611020f0f1302040415020000170201010e02060f1002030612020f0f140205041602000018020101190201022a01011a01ff1b030841001c0101235c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000240300000025020501260200002703005a002906007f1d0f61702c030000012e01633001033101003202000133060001299f936a3428030152b8fe3f332b8845aaf1123f01000040bf665e174673684140010000804000c06a46f2d24d3e3501ff360201014401003a0102";

    pub fn captured_state_body() -> Vec<u8> {
        (0..CAPTURED_STATE_BODY.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&CAPTURED_STATE_BODY[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn parses_captured_state_packet() {
        let body = captured_state_body();
        let (remaining, packet) =
            A3956StateUpdatePacket::take::<VerboseError<_>>(&body).expect("should parse");
        assert!(remaining.is_empty());
        let info = packet.info();
        assert_eq!(info.serial_number, "1203007F1D22341E");
        assert_eq!(info.firmware_left, "05.51");
        assert_eq!(info.firmware_right, "05.51");
        assert_eq!(info.battery_left, 93);
        assert_eq!(info.battery_right, 92);
        // round trip
        assert_eq!(packet.body(), body);
    }

    #[test]
    fn parses_easy_chat_event() {
        // Captured on the Liberty 5 Pro: start = 00 01, end = 00 00 (checksum already stripped).
        let (_, start) = EasyChatEvent::take::<VerboseError<_>>(&[0x00, 0x01]).unwrap();
        assert!(start.is_active);
        let (_, end) = EasyChatEvent::take::<VerboseError<_>>(&[0x00, 0x00]).unwrap();
        assert!(!end.is_active);
    }
}
