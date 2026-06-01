class EventSink {
  void push(Map<String, Object> event) {}
}

/// Test-data helper: every call site varies only the string-literal
/// arguments. The call-shape clusters but the variation is test data,
/// not duplication (#70).
void recordEvent(String name, Map<String, String> tags, String id) {}
