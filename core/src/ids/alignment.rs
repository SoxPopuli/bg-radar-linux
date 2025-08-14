crate::int_enum! {
    pub enum Alignment: u8 {
        None = 0x00,
        LawfulGood = 0x11,
        LawfulNeutral = 0x12,
        LawfulEvil = 0x13,
        NeutralGood = 0x21,
        Neutral = 0x22,
        NeutralEvil = 0x23,
        ChaoticGood = 0x31,
        ChaoticNeutral = 0x32,
        ChaoticEvil = 0x33,
        MaskGood = 0x01,
        MaskGENeutral = 0x02,
        MaskEvil = 0x03,
        MaskLawful = 0x10,
        MaskLCNeutral = 0x20,
        MaskChaotic = 0x30,
    }
}
