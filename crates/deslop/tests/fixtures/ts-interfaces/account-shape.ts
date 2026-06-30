export interface AccountRecord {
  readonly key: string;
  label: string;
  contact?: string;
  scopes: ReadonlyArray<string>;
  settings: {
    mode: "light" | "dark";
    alerts: boolean;
  };
  touchedAt: Date | null;
}
