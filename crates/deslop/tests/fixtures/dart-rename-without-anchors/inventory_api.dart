import 'http_shim.dart';

class InventoryApi {
  InventoryApi(this.http, this.uid);

  final HttpShim http;
  final String uid;

  Future<List<String>> fetchStockLocations() async {
    final stockResponse = await http
        .getMethod<List<Object?>>('/catalog/$uid/settings/stock-locations');
    final stockRows = stockResponse.data ?? const <Object?>[];
    final stockLabels = stockRows.map((stockRow) => stockRow.toString()).toList();
    stockLabels.sort((stockLeft, stockRight) => stockLeft.compareTo(stockRight));
    return stockLabels.cast<String>();
  }

  Future<List<String>> fetchReorderPoints() async {
    final reorderResponse = await http
        .getMethod<List<Object?>>('/catalog/$uid/settings/reorder-points');
    final reorderRows = reorderResponse.data ?? const <Object?>[];
    final reorderLabels = reorderRows.map((reorderRow) => reorderRow.toString()).toList();
    reorderLabels.sort((reorderLeft, reorderRight) => reorderLeft.compareTo(reorderRight));
    return reorderLabels.cast<String>();
  }

  Future<List<String>> fetchSupplierCodes() async {
    final supplierResponse = await http
        .getMethod<List<Object?>>('/catalog/$uid/settings/supplier-codes');
    final supplierRows = supplierResponse.data ?? const <Object?>[];
    final supplierLabels = supplierRows.map((supplierRow) => supplierRow.toString()).toList();
    supplierLabels.sort((supplierLeft, supplierRight) => supplierLeft.compareTo(supplierRight));
    return supplierLabels.cast<String>();
  }

  Future<List<String>> fetchAuditTrails() async {
    final auditResponse = await http
        .getMethod<List<Object?>>('/catalog/$uid/settings/audit-trails');
    final auditRows = auditResponse.data ?? const <Object?>[];
    final auditLabels = auditRows.map((auditRow) => auditRow.toString()).toList();
    auditLabels.sort((auditLeft, auditRight) => auditLeft.compareTo(auditRight));
    return auditLabels.cast<String>();
  }

}
