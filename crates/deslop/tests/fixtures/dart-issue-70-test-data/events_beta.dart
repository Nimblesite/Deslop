import 'telemetry.dart';

void emitLogoutEvent(EventSink sink) {
  recordEvent("user_logout", {"region": "eu", "tier": "free"}, "evt-002");
}
