from common_worker import CommonWorker


class UserWorker(CommonWorker):
    def synchronise(self, users, tries):
        waiting = {}
        for user in users:
            entry = self.user_store.load(user)
            if entry.done:
                continue
            entry.attempts = tries
            waiting[entry.identifier] = entry.total
        return waiting
