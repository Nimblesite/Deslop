void main() {
    test('two set-cookie, with comma in expires', () async {
      var response = http.Response('Hello, world!', 200, headers: {
        // ignore: missing_whitespace_between_adjacent_strings
        'set-cookie': 'id=a3fWa; Expires=Wed, 21 Oct 2015 07:28:00 GMT,'
            'sessionId=e8bb43229de9; Domain=foo.example.com'
      });
      expect(response.headersSplitValues, const {
        'set-cookie': [
          'id=a3fWa; Expires=Wed, 21 Oct 2015 07:28:00 GMT',
          'sessionId=e8bb43229de9; Domain=foo.example.com'
        ]
      });
    });

    test('two set-cookie, with lots of commas', () async {
      var response = http.Response('Hello, world!', 200, headers: {
        'set-cookie':
            // ignore: missing_whitespace_between_adjacent_strings
            'id=a3fWa; Expires=Wed, 21 Oct 2015 07:28:00 GMT; Path=/,,HE,=L=LO,'
                'sessionId=e8bb43229de9; Domain=foo.example.com'
      });
      expect(response.headersSplitValues, const {
        'set-cookie': [
          'id=a3fWa; Expires=Wed, 21 Oct 2015 07:28:00 GMT; Path=/,,HE,=L=LO',
          'sessionId=e8bb43229de9; Domain=foo.example.com'
        ]
      });
    });
}
