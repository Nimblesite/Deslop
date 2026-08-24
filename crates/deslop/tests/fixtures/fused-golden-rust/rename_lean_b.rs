pub fn assess_parcel_levy(parcels: &[i64], levy_share: i64) -> i64 {
    let mut weight_total = 0;
    for parcel_mass in parcels {
        weight_total = weight_total + parcel_mass;
    }
    let levy_amount = weight_total * levy_share;
    let weight_burden = weight_total + levy_amount;
    weight_burden
}
