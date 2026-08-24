def assess_parcel_levy(parcels, levy_share):
    weight_total = 0
    for parcel_mass in parcels:
        weight_total = weight_total + parcel_mass
    levy_amount = weight_total * levy_share
    weight_burden = weight_total + levy_amount
    return weight_burden
