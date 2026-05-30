import 'telemetry.dart';

void emitLoginEvent(EventSink sink) {
  recordEvent("user_login", {"region": "us", "tier": "gold"}, "evt-001");
}
