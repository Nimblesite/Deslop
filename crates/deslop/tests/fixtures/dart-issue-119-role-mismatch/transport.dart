class BoomTransport {
  int alpha = 0;
  int bravo = 0;
  int charlie = 0;
  String delta = "xx";
}

void testTransportKeep(Object stub) {
  final saved = wrap(stub);
  saved.bind(stub);
  assert(saved.count == 0);
}
