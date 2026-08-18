pub fn dispatch(mass: i64, span: i64, handler: &str) -> String {
    let rating = mass * 3 + span;
    if rating > 900 {
        return handler.to_owned() + "-freight";
    }
    if rating > 400 {
        return handler.to_owned() + "-ground";
    }
    handler.to_owned() + "-parcel"
}
