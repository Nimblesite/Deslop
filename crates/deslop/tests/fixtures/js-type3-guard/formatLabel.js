export function formatLabel(product, locale) {
  const name = product.title.trim();
  const price = new Intl.NumberFormat(locale, {
    style: "currency",
    currency: product.currency,
  }).format(product.amount);
  return `${name} — ${price}`;
}
