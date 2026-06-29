export interface UserProfile {
  readonly id: string;
  displayName: string;
  email?: string;
  roles: ReadonlyArray<string>;
  preferences: {
    theme: "light" | "dark";
    notifications: boolean;
  };
  lastSeenAt: Date | null;
}
