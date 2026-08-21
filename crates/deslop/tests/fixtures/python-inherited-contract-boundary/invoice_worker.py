from common_worker import CommonWorker


class InvoiceWorker(CommonWorker):
    def synchronise(self, orders, attempts):
        pending = []
        while orders:
            order = orders.pop()
            record = self.order_repo.fetch(order)
            if record.attempts > attempts:
                pending.append(record.identifier)
            else:
                self.order_repo.retry(record)
        return pending
