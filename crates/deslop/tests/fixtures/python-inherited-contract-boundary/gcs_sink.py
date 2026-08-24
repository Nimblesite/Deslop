from ledger_base import LedgerSink


class GcsLedgerSink(LedgerSink):
    def record_entry(self, entry, ledger_id, stamped_at):
        blob = self.blobs[ledger_id]
        payload = blob.encode(entry)
        if payload is None:
            return None
        payload.stamped_at = stamped_at
        blob.upload(payload)
        return payload.generation
