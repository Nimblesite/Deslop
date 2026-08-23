from abc import ABC, abstractmethod


class LedgerSink(ABC):
    @abstractmethod
    def record_entry(self, entry, ledger_id, stamped_at):
        ...
