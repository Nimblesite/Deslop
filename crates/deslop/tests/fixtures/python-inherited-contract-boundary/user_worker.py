from common_worker import CommonWorker


class UserWorker(CommonWorker):
    def synchronise(self, users, tries):
        waiting = []
        while users:
            user = users.pop()
            entry = self.user_store.load(user)
            if entry.attempts > tries:
                waiting.append(entry.identifier)
            else:
                self.user_store.requeue(entry)
        return waiting
