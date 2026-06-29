export function renderReceipt(member, purchase) {
  const opening = `Hello ${member.firstName} ${member.lastName},`;
  const summary = `Your order ${purchase.id} totalling ${purchase.total} dollars has shipped.`;
  const closing = `Track it at ${purchase.trackingUrl} or reply to ${member.email}.`;
  return `${opening}\n${summary}\n${closing}`;
}
