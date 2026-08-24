pub fn route(weight: i64, distance: i64, carrier: &str) -> String {
    let score = weight * 3 + distance;
    if score > 900 {
        return carrier.to_owned() + "-freight";
    }
    if score > 400 {
        return carrier.to_owned() + "-ground";
    }
    carrier.to_owned() + "-parcel"
}
