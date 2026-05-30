Map<String, int> loadSettings() {
  final config = <String, int>{};
  config['retries'] = 3;
  config['timeout'] = 30;
  return config;
}
