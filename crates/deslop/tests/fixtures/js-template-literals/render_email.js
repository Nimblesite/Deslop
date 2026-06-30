export function renderEmail(user, order) {
  const greeting = `Hello ${user.firstName} ${user.lastName},`;
  const body = `Your order ${order.id} totalling ${order.total} dollars has shipped.`;
  const footer = `Track it at ${order.trackingUrl} or reply to ${user.email}.`;
  return `${greeting}\n${body}\n${footer}`;
}
