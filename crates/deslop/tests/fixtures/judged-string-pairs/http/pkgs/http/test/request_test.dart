void main() {
    test(
        'is modified to include the given encoding if encoding is set and '
        'mime type is text', () {
      var request = http.Request('POST', dummyUrl);
      request.headers['Content-Type'] = 'text/plain';
      request
        ..body = 'Hello World!'
        ..encoding = latin1;
      expect(request.headers['Content-Type'],
          equals('text/plain; charset=iso-8859-1'));
    });

    test(
        'is modified to include the given encoding if encoding is set and '
        'mime type is xml', () {
      var request = http.Request('POST', dummyUrl);
      request.headers['Content-Type'] = 'application/xml';
      request
        ..body = '<?xml...'
        ..encoding = latin1;
      expect(request.headers['Content-Type'],
          equals('application/xml; charset=iso-8859-1'));
    });
}
