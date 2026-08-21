from ledger_base import LedgerSink


class S3LedgerSink(LedgerSink):
    def record_entry(self, entry, ledger_id, stamped_at):
        bucket = self.buckets[ledger_id]
        payload = bucket.serialise(entry)
        if payload is None:
            return None
        payload.stamped_at = stamped_at
        bucket.put_object(payload)
        return payload.etag
