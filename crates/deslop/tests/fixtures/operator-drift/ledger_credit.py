def post_credit(entries, rate, floor, ceiling):
    running = 0
    adjusted = []
    for entry in entries:
        scaled = entry * rate
        shifted = scaled + floor
        clamped = min(shifted, ceiling)
        adjusted.append(clamped)
        running = running + clamped
    average = running / max(len(adjusted), 1)
    spread = max(adjusted) - min(adjusted)
    summary = {"total": running, "average": average, "spread": spread}
    return summary
