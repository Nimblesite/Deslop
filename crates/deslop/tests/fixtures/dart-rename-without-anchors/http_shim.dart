class ApiResponse<T> {
  ApiResponse(this.data);
  final T? data;
}

class HttpShim {
  Future<ApiResponse<T>> getMethod<T>(String path) async =>
      throw UnsupportedError(path);
}
