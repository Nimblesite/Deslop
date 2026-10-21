// One `<select>` renderer for every facet axis in the report webview
// ([FACET-REPORT-WEBVIEW]). A single implementation so the axes can
// never drift in markup or null-handling ("" ⇄ null = no filter).
// [METRICS-TOTAL-INCLUDE] Excluded kinds render muted, never removed.

const EXCLUDED_OPTION_CLASS = "kind-option--excluded";

interface FilterOption<T extends string> {
  label: string;
  value: T | null;
  /** [METRICS-TOTAL-INCLUDE] Excluded kinds render muted — greyed, still selectable. */
  excluded?: boolean;
}

export function FilterSelect<T extends string>(props: {
  options: readonly FilterOption<T>[];
  value: T | null;
  onChange: (value: T | null) => void;
}) {
  return (
    <select
      value={props.value ?? ""}
      onChange={(event) => {
        const raw = (event.currentTarget as HTMLSelectElement).value;
        props.onChange(raw === "" ? null : (raw as T));
      }}
    >
      {props.options.map((option) => (
        <option
          key={option.label}
          label={option.label}
          value={option.value ?? ""}
          class={option.excluded ? EXCLUDED_OPTION_CLASS : undefined}
        >
          {option.label}
        </option>
      ))}
    </select>
  );
}
