from common_worker import CommonWorker


class InvoiceWorker(CommonWorker):
    def synchronise(self, order, attempts):
        repo = self.order_repo[order]
        record = repo.fetch(order)
        if record is None:
            return None
        record.attempts = attempts
        repo.save(record)
        return record.identifier
