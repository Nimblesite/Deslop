// Fixture for GH #197 — an in-class sibling-method family in ONE file.
//
// Vendored verbatim from meilisearch/meilisearch-dart `lib/src/index.dart`
// @ main (the issue's reproduction), lines 487-789: the index-settings
// get/reset/update method family. Each method shares a skeleton (await an
// HTTP call, decode/forward the body) but targets a different endpoint
// literal and return type, so after structural normalisation they fuse at
// `structural=1.00` with no shared tokens (`token_jaccard=0.00`) and no
// embedding support — a `structural_only` family that is the public REST
// API surface, not extract-worthy duplication.
//
// Before #197 these single-file families ranked as the #1/#2 top offenders
// (be951a686525 size=7 w=738, 7f363063109f size=8 w=574) because
// `is_scaffolding_structural_only` only demoted clusters spread across 3+
// files. They must NOT surface as top offenders. Scaffolding below the
// marker is the minimum needed to make the slice parse as a Dart class.

class Task {}

class IndexSettings {}

class Embedder {}

class HttpResponse<T> {
  HttpResponse(this.data);
  final T? data;
}

class HttpClient {
  Future<HttpResponse<T>> getMethod<T>(String path) async =>
      throw UnimplementedError();
  Future<HttpResponse<T>> deleteMethod(String path) async =>
      throw UnimplementedError();
  Future<HttpResponse<T>> putMethod<T>(String path, {Object? data}) async =>
      throw UnimplementedError();
}

class MeiliSearchIndex {
  MeiliSearchIndex(this.uid, this.http);

  final String uid;
  final HttpClient http;

  Future<Task> _getTask(Future<Object?> future) async => Task();

  // ───────── vendored meilisearch-dart settings family ─────────
  Future<IndexSettings> getSettings() async {
    final response =
        await http.getMethod<Map<String, Object?>>('/indexes/$uid/settings');

    return IndexSettings.fromMap(response.data!);
  }

  /// Reset the settings of the index.
  /// All settings will be reset to their default value.
  Future<Task> resetSettings() async {
    return await _getTask(http.deleteMethod('/indexes/$uid/settings'));
  }

  /// Update the settings of the index. Any parameters not provided in the body will be left unchanged.
  Future<Task> updateSettings(IndexSettings settings) async {
    return await _getTask(http.patchMethod(
      '/indexes/$uid/settings',
      data: settings.toMap(),
    ));
  }

  /// Get filterable attributes of the index.
  Future<List<String>> getFilterableAttributes() async {
    final response = await http.getMethod<List<Object?>>(
        '/indexes/$uid/settings/filterable-attributes');

    return response.data!.cast<String>();
  }

  /// Reset filterable attributes of the index.
  Future<Task> resetFilterableAttributes() async {
    return await _getTask(
        http.deleteMethod('/indexes/$uid/settings/filterable-attributes'));
  }

  /// Update filterable attributes of the index.
  Future<Task> updateFilterableAttributes(
    List<String> filterableAttributes,
  ) async {
    return await _getTask(
      http.putMethod(
        '/indexes/$uid/settings/filterable-attributes',
        data: filterableAttributes,
      ),
    );
  }

  /// Get the displayed attributes of the index.
  Future<List<String>> getDisplayedAttributes() async {
    final response = await http.getMethod<List<Object?>>(
        '/indexes/$uid/settings/displayed-attributes');

    return response.data!.cast<String>();
  }

  /// Reset the displayed attributes of the index.
  Future<Task> resetDisplayedAttributes() async {
    return await _getTask(
      http.deleteMethod('/indexes/$uid/settings/displayed-attributes'),
    );
  }

  /// Update the displayed attributes of the index.
  Future<Task> updateDisplayedAttributes(
    List<String> displayedAttributes,
  ) async {
    return await _getTask(
      http.putMethod(
        '/indexes/$uid/settings/displayed-attributes',
        data: displayedAttributes,
      ),
    );
  }

  /// Get the distinct attribute for the index.
  Future<String?> getDistinctAttribute() async {
    final response = await http
        .getMethod<String?>('/indexes/$uid/settings/distinct-attribute');

    return response.data;
  }

  /// Reset the distinct attribute for the index.
  Future<Task> resetDistinctAttribute() async {
    return await _getTask(
      http.deleteMethod('/indexes/$uid/settings/distinct-attribute'),
    );
  }

  /// Update the distinct attribute for the index.
  Future<Task> updateDistinctAttribute(String distinctAttribute) async {
    return await _getTask(
      http.putMethod(
        '/indexes/$uid/settings/distinct-attribute',
        data: '"$distinctAttribute"',
      ),
    );
  }

  /// Get ranking rules of the index.
  Future<List<String>> getRankingRules() async {
    final response = await http
        .getMethod<List<Object?>>('/indexes/$uid/settings/ranking-rules');

    return response.data!.cast<String>();
  }

  /// Reset ranking rules of the index.
  Future<Task> resetRankingRules() async {
    return await _getTask(
      http.deleteMethod('/indexes/$uid/settings/ranking-rules'),
    );
  }

  /// Update ranking rules of the index.
  Future<Task> updateRankingRules(List<String> rankingRules) async {
    return await _getTask(
      http.putMethod(
        '/indexes/$uid/settings/ranking-rules',
        data: rankingRules,
      ),
    );
  }

  /// Get separator tokens of the index.
  Future<List<String>> getSeparatorTokens() async {
    final response = await http
        .getMethod<List<Object?>>('/indexes/$uid/settings/separator-tokens');

    return response.data!.cast<String>();
  }

  /// Reset separator tokens of the index.
  Future<Task> resetSeparatorTokens() async {
    return await _getTask(
      http.deleteMethod('/indexes/$uid/settings/separator-tokens'),
    );
  }

  /// Update separator tokens of the index.
  Future<Task> updateSeparatorTokens(List<String> separatorTokens) async {
    return await _getTask(
      http.putMethod(
        '/indexes/$uid/settings/separator-tokens',
        data: separatorTokens,
      ),
    );
  }

  /// Get non separator tokens of the index.
  Future<List<String>> getNonSeparatorTokens() async {
    final response = await http.getMethod<List<Object?>>(
        '/indexes/$uid/settings/non-separator-tokens');

    return response.data!.cast<String>();
  }

  /// Reset separator tokens of the index.
  Future<Task> resetNonSeparatorTokens() async {
    return await _getTask(
      http.deleteMethod('/indexes/$uid/settings/non-separator-tokens'),
    );
  }

  /// Update separator tokens of the index.
  Future<Task> updateNonSeparatorTokens(List<String> nonSeparatorTokens) async {
    return await _getTask(
      http.putMethod(
        '/indexes/$uid/settings/non-separator-tokens',
        data: nonSeparatorTokens,
      ),
    );
  }

  /// Get searchable attributes of the index.
  Future<List<String>> getSearchableAttributes() async {
    final response = await http.getMethod<List<Object?>>(
      '/indexes/$uid/settings/searchable-attributes',
    );

    return response.data!.cast<String>();
  }

  /// Reset searchable attributes of the index.
  Future<Task> resetSearchableAttributes() async {
    return await _getTask(
      http.deleteMethod('/indexes/$uid/settings/searchable-attributes'),
    );
  }

  /// Update the searchable attributes of the index.
  Future<Task> updateSearchableAttributes(
      List<String> searchableAttributes) async {
    return await _getTask(
      http.putMethod(
        '/indexes/$uid/settings/searchable-attributes',
        data: searchableAttributes,
      ),
    );
  }

  /// Get the embedders settings of a Meilisearch index.
  @RequiredMeiliServerVersion('1.6.0')
  Future<Map<String, Embedder>?> getEmbedders() async {
    final response = await http
        .getMethod<Map<String, Object?>>('/indexes/$uid/settings/embedders');

    return response.data?.map(
        (k, v) => MapEntry(k, Embedder.fromMap(v as Map<String, Object?>)));
  }

  /// Update the embedders settings. Overwrite the old settings.
  @RequiredMeiliServerVersion('1.6.0')
  Future<Task> updateEmbedders(Map<String, Embedder>? embedders) async {
    return await _getTask(
      http.putMethod(
        '/indexes/$uid/settings/embedders',
        data: embedders?.map((k, v) => MapEntry(k, v.toMap())),
      ),
    );
  }

  /// Reset the embedders settings to its default value
  @RequiredMeiliServerVersion('1.6.0')
  Future<Task> resetEmbedders() async {
    return await _getTask(
      http.deleteMethod('/indexes/$uid/settings/embedders'),
    );
  }

  //
  // StopWords endpoints
  //

  /// Get stop words of the index.
  Future<List<String>> getStopWords() async {
    final response = await http
        .getMethod<List<Object?>>('/indexes/$uid/settings/stop-words');

    return response.data!.cast<String>();
  }

  /// Reset stop words of the index.
  Future<Task> resetStopWords() async {
    return await _getTask(
        http.deleteMethod('/indexes/$uid/settings/stop-words'));
  }

  /// Update stop words of the index
  Future<Task> updateStopWords(List<String> stopWords) async {
    return await _getTask(
        http.putMethod('/indexes/$uid/settings/stop-words', data: stopWords));
  }

  //
  // Synonyms endpoints
  //

  /// Get synonyms of the index.
  Future<Map<String, List<String>>> getSynonyms() async {
    final response = await http
        .getMethod<Map<String, Object?>>('/indexes/$uid/settings/synonyms');

    return response.data!
        .map((key, value) => MapEntry(key, (value as List).cast<String>()));
  }

  /// Reset synonyms of the index.
  Future<Task> resetSynonyms() async {
    return await _getTask(http.deleteMethod('/indexes/$uid/settings/synonyms'));
  }

  /// Update synonyms of the index
  Future<Task> updateSynonyms(Map<String, List<String>> synonyms) async {
    return await _getTask(
        http.putMethod('/indexes/$uid/settings/synonyms', data: synonyms));
  }

  //
  // Sortable Attributes endpoints
  //

  /// Get sortable attributes of the index.
  Future<List<String>> getSortableAttributes() async {
    final response = await http
        .getMethod<List<Object?>>('/indexes/$uid/settings/sortable-attributes');

    return response.data!.cast<String>();
  }

  /// Reset sortable attributes of the index.
  Future<Task> resetSortableAttributes() async {
    return await _getTask(
        http.deleteMethod('/indexes/$uid/settings/sortable-attributes'));
  }

  /// Update sortable attributes of the index.
  Future<Task> updateSortableAttributes(List<String> sortableAttributes) async {
    return _getTask(
      http.putMethod(
        '/indexes/$uid/settings/sortable-attributes',
        data: sortableAttributes,
      ),
}
