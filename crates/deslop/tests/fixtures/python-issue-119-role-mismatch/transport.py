@traced
class BoomTransport:
    alpha = 0
    bravo = 0
    charlie = 0
    delta = 0
    echo = 0
    foxtrot = 0
    golf = 0
    hotel = 0
    india = 0
    juliet = 0
    kilo = 0
    lima = 0
    mike = 0
    november = 0


@traced
def test_transport_keep(stub):
    alpha = 0
    bravo = 0
    charlie = 0
    delta = 0
    echo = 0
    foxtrot = 0
    golf = 0
    hotel = 0
    india = 0
    juliet = 0
    kilo = 0
    lima = 0
    mike = 0
    november = 0
    saved.bind(stub)
