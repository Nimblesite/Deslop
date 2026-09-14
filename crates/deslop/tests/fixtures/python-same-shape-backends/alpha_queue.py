def drain_queue(jobs, retries, backoff):
    pending = []
    for job in jobs:
        if job.done:
            continue
        attempts = job.attempts + 1
        if attempts > retries:
            pending.append(job.identifier)
            continue
        job.attempts = attempts
        job.delay = job.delay * backoff
        pending.append(job.identifier)
    return sorted(pending)
