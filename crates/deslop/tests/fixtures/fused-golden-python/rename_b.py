def dispatch(mass, span, handler):
    rating = mass * 3 + span
    if rating > 900:
        return handler + "-freight"
    if rating > 400:
        return handler + "-ground"
    return handler + "-parcel"
