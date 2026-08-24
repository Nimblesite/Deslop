import 'http_shim.dart';

class CatalogApi {
  CatalogApi(this.http, this.uid);

  final HttpShim http;
  final String uid;

  Future<List<String>> fetchDisplayBanners() async {
    final bannerResponse = await http
        .getMethod<List<Object?>>('/catalog/$uid/settings/display-banners');
    final bannerRows = bannerResponse.data ?? const <Object?>[];
    final bannerLabels = bannerRows.map((bannerRow) => bannerRow.toString()).toList();
    bannerLabels.sort((bannerLeft, bannerRight) => bannerLeft.compareTo(bannerRight));
    return bannerLabels.cast<String>();
  }

  Future<List<String>> fetchPricingTiers() async {
    final pricingResponse = await http
        .getMethod<List<Object?>>('/catalog/$uid/settings/pricing-tiers');
    final pricingRows = pricingResponse.data ?? const <Object?>[];
    final pricingLabels = pricingRows.map((pricingRow) => pricingRow.toString()).toList();
    pricingLabels.sort((pricingLeft, pricingRight) => pricingLeft.compareTo(pricingRight));
    return pricingLabels.cast<String>();
  }

  Future<List<String>> fetchSeasonLabels() async {
    final seasonResponse = await http
        .getMethod<List<Object?>>('/catalog/$uid/settings/season-labels');
    final seasonRows = seasonResponse.data ?? const <Object?>[];
    final seasonLabels = seasonRows.map((seasonRow) => seasonRow.toString()).toList();
    seasonLabels.sort((seasonLeft, seasonRight) => seasonLeft.compareTo(seasonRight));
    return seasonLabels.cast<String>();
  }

}
