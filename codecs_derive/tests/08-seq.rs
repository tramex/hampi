#![allow(non_camel_case_types, dead_code)]

use asn_codecs_derive::AperCodec;

#[derive(Debug, AperCodec)]
#[asn(type = "INTEGER", lb = 0i128, ub = 65535i128)]
pub struct ProtocolIE_ID(u16);

#[derive(Debug, AperCodec)]
#[asn(type = "ENUMERATED", lb = 0i128, ub = 2i128)]
pub struct Criticality(u8);
impl Criticality {
    const REJECT: u8 = 0u8;
    const IGNORE: u8 = 1u8;
    const NOTIFY: u8 = 2u8;
}

#[derive(Debug, AperCodec)]
#[asn(type = "SEQUENCE", extensible = false)]
pub struct Bearers_SubjectToEarlyStatusTransferListItem {
    pub id: ProtocolIE_ID,
    #[asn(optional_idx = 1)]
    pub criticality: Option<Criticality>,
}

fn main() {
    eprintln!("Sequence");
}
