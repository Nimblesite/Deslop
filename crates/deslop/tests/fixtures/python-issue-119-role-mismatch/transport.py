@traced
class BoomTransport:
    alpha = 0
    bravo = 0
    charlie = 0
    delta = "xx"


@traced
def test_transport_keep(stub):
    saved = wrap(stub)
    saved.bind(stub)
    assert saved.n == 0
