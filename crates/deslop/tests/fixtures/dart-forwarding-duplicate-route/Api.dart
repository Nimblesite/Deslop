// A five-member REST wrapper family with a copy-paste bug in it.
//
// `resetAlpha`, `resetBeta` and `resetGamma` are ordinary API surface —
// each forwards one client call, each targets a different route. On
// their own they are exactly the family `[RANK-STRUCTURAL-ONLY]` hides.
//
// `resetDelta` and `resetEpsilon` both DELETE `/indexes/dup/settings`.
// Their bodies are byte-for-byte identical, so one of these two calls is
// dead or one of them is aimed at the wrong route. That is a real
// finding, and hiding the family would erase it.
//
// The declarations differ only in the method name, so the reported
// windows never compare equal — the duplication is visible only in the
// proven bodies. This fixture exists because a window-level distinctness
// check silently passes here and hides the bug
// ([RANK-STRUCTURAL-ONLY-FORWARDING]).
class Api {
  Future<Task> resetAlpha() async {
    return await _getTask(http.deleteMethod('/indexes/alpha/settings'));
  }

  Future<Task> resetBeta() async {
    return await _getTask(http.deleteMethod('/indexes/beta/settings'));
  }

  Future<Task> resetGamma() async {
    return await _getTask(http.deleteMethod('/indexes/gamma/settings'));
  }

  Future<Task> resetDelta() async {
    return await _getTask(http.deleteMethod('/indexes/dup/settings'));
  }

  Future<Task> resetEpsilon() async {
    return await _getTask(http.deleteMethod('/indexes/dup/settings'));
  }

  Future<Task> _getTask(Future<Object?> f) async => Task();
}

class Task {}
