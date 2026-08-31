def reconcile_alpha(record, rate):
    total = record.opening
    total = total + record.credit_01 * rate
    total = total + record.credit_02 * rate
    total = total + record.credit_03 * rate
    total = total + record.credit_04 * rate
    total = total + record.credit_05 * rate
    total = total + record.credit_06 * rate
    total = total + record.credit_07 * rate
    total = total + record.credit_08 * rate
    total = total + record.credit_09 * rate
    total = total + record.credit_10 * rate
    total = total + record.credit_11 * rate
    total = total + record.credit_12 * rate
    total = total + record.credit_13 * rate
    total = total + record.credit_14 * rate
    if record.active:
        total = total + record.credit_15 * rate
    total = total + record.credit_16 * rate
    total = total + record.credit_17 * rate
    total = total + record.credit_18 * rate
    total = total + record.credit_19 * rate
    total = total + record.credit_20 * rate
    total = total + record.credit_21 * rate
    total = total + record.credit_22 * rate
    total = total + record.credit_23 * rate
    total = total + record.credit_24 * rate
    return total
