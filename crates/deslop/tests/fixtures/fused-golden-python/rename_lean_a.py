def calibrate_sensor_drift(readings, gain_factor):
    drift_sum = 0
    for reading_value in readings:
        drift_sum = drift_sum + reading_value
    gain_adjusted = drift_sum * gain_factor
    drift_score = drift_sum + gain_adjusted
    return drift_score
