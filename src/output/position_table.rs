use tabled::{Table, Tabled};

#[derive(Tabled)]
pub struct PositionRow {
    #[tabled(rename = "Pool")]
    pub pool_id:     String,
    #[tabled(rename = "Tick Lower")]
    pub tick_lower:  i32,
    #[tabled(rename = "Tick Upper")]
    pub tick_upper:  i32,
    #[tabled(rename = "Liquidity")]
    pub liquidity:   u128,
    #[tabled(rename = "Fee (bps)")]
    pub fee_rate_bps: u16,
}

pub fn print_position_inventory(positions: &[PositionRow]) {
    if positions.is_empty() {
        println!("No positions found.");
        return;
    }
    println!("{}", Table::new(positions));
}