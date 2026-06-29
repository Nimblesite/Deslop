export function Avatar({ user, size = "md" }) {
  const initials = user.name
    .split(" ")
    .map((part) => part[0])
    .join("")
    .toUpperCase();
  return (
    <figure className={`avatar avatar-${size}`} title={user.name}>
      {user.imageUrl ? <img src={user.imageUrl} alt={user.name} /> : <span>{initials}</span>}
    </figure>
  );
}
