pub fn calibrate_sensor_drift(readings: &[i64], gain_factor: i64) -> i64 {
    let mut drift_sum = 0;
    for reading_value in readings {
        drift_sum = drift_sum + reading_value;
    }
    let gain_adjusted = drift_sum * gain_factor;
    let drift_score = drift_sum + gain_adjusted;
    drift_score
}
