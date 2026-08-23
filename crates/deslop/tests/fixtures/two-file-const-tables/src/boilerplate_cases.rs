// Test-data blobs for the boilerplate suite. Every payload is a snippet
// of a different language, held here so the suite can feed it to the
// scanner. Nothing in this file is shared with `defaults_cases.rs`.

const CSHARP_ALPHA: &str = r"
public sealed class OrderGateway
{
    private readonly HttpClient _client;

    public OrderGateway(HttpClient client)
    {
        _client = client;
    }

    public async Task<Order> FetchAsync(int orderId)
    {
        var response = await _client.GetAsync($\"/orders/{orderId}\");
        return await response.Content.ReadFromJsonAsync<Order>();
    }
}
";

const RUST_BETA: &str = r"
pub struct Ledger {
    entries: Vec<Entry>,
}

impl Ledger {
    pub fn post(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn balance(&self) -> i64 {
        self.entries.iter().map(|entry| entry.amount).sum()
    }
}
";

const PYTHON_GAMMA: &str = r"
class RetryPolicy:
    def __init__(self, attempts, backoff):
        self.attempts = attempts
        self.backoff = backoff

    def next_delay(self, attempt):
        return self.backoff * (2 ** attempt)
";

const JS_ALPHA: &str = r"
export function createStore(reducer, preloadedState) {
  let state = preloadedState;
  const listeners = [];
  return {
    getState: () => state,
    dispatch(action) {
      state = reducer(state, action);
      listeners.forEach((listener) => listener());
    },
  };
}
";

const TS_DELTA: &str = r"
export interface Invoice {
  readonly id: string;
  readonly issuedOn: Date;
  readonly lines: ReadonlyArray<InvoiceLine>;
}

export type InvoiceLine = {
  description: string;
  quantity: number;
  unitPrice: number;
};
";

const GO_EPSILON: &str = r"
package ledger

func Reconcile(entries []int64, floor int64) int64 {
	var balance int64
	for _, entry := range entries {
		if entry > floor {
			balance += entry * 2
		} else {
			balance -= entry / 2
		}
	}
	return balance
}
";
