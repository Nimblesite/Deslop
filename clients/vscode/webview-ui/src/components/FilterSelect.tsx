// One `<select>` renderer for every facet axis in the report webview
// ([FACET-REPORT-WEBVIEW]). A single implementation so the axes can
// never drift in markup or null-handling ("" ⇄ null = no filter).

interface FilterOption<T extends string> {
  label: string;
  value: T | null;
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
        <option key={option.label} value={option.value ?? ""}>
          {option.label}
        </option>
      ))}
    </select>
  );
}
