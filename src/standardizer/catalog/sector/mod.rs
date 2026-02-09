pub mod banks;
pub mod insurance;
pub mod reits;

use crate::standardizer::catalog::MetricDefinition;

/// Determine the sector from a SIC code and return relevant extra metrics.
pub fn sector_definitions(sic: Option<&str>) -> Vec<MetricDefinition> {
    let sic = match sic {
        Some(s) => s,
        None => return Vec::new(),
    };

    let sic_num: u32 = sic.parse().unwrap_or(0);

    match sic_num {
        // National & state commercial banks, savings institutions, credit unions
        6000..=6099 => banks::definitions(),
        // Insurance
        6300..=6399 | 6400..=6411 => insurance::definitions(),
        // REITs
        6500..=6553 | 6798 => reits::definitions(),
        _ => Vec::new(),
    }
}
