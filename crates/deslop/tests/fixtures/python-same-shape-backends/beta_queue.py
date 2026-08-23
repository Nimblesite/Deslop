def drain_queue(tasks, limit, factor):
    waiting = []
    for task in tasks:
        if task.done:
            continue
        tries = task.attempts + 1
        if tries > limit:
            waiting.append(task.identifier)
            continue
        task.attempts = tries
        task.delay = task.delay * factor
        waiting.append(task.identifier)
    return sorted(waiting)
