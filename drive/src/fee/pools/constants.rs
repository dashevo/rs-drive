pub const KEY_STORAGE_FEE_POOL: &str = "s";
pub const KEY_GENESIS_TIME: &str = "g";

pub const EPOCH_CHANGE_TIME: i64 = 1576800000;

pub const FEE_DISTRIBUTION_TABLE: [f64; 50] = [
    0.050000,
    0.048000,
    0.046000,
    0.044000,
    0.042000,
    0.040000,
    0.038500,
    0.037000,
    0.035500,
    0.034000,
    0.032500,
    0.031000,
    0.029500,
    0.028500,
    0.027500,
    0.026500,
    0.025500,
    0.024500,
    0.023500,
    0.022500,
    0.021500,
    0.020500,
    0.019500,
    0.018750,
    0.018000,
    0.017250,
    0.016500,
    0.015750,
    0.015000,
    0.014250,
    0.013500,
    0.012750,
    0.012000,
    0.011250,
    0.010500,
    0.009750,
    0.009000,
    0.008250,
    0.007500,
    0.006750,
    0.006000,
    0.005250,
    0.004750,
    0.004250,
    0.003750,
    0.003250,
    0.002750,
    0.002250,
    0.001750,
    0.0012500000000004,
];

pub const MN_REWARD_SHARES_CONTRACT_ID: [u8; 32] = [
    0x0c, 0xac, 0xe2, 0x05, 0x24, 0x66, 0x93, 0xa7, 0xc8, 0x15, 0x65, 0x23, 0x62, 0x0d, 0xaa, 0x93,
    0x7d, 0x2f, 0x22, 0x47, 0x93, 0x44, 0x63, 0xee, 0xb0, 0x1f, 0xf7, 0x21, 0x95, 0x90, 0x95, 0x8c,
];

pub const MN_REWARD_SHARES_DOCUMENT_TYPE: &'static str = "rewardShare";

#[cfg(test)]
mod tests {
    #[test]
    fn test_distribution_table_sum() {
        assert_eq!(super::FEE_DISTRIBUTION_TABLE.iter().sum::<f64>(), 1.0);
    }
}
