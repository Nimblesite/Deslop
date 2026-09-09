public class Gamma {
  public string Describe(string[] parts) {
    var builder = new System.Text.StringBuilder();
    foreach (var part in parts) {
      if (part.Length == 0) {
        continue;
      }
      builder.Append(part.ToUpperInvariant());
      builder.Append(", ");
    }
    while (builder.Length > 40) {
      builder.Length -= 1;
    }
    return builder.ToString();
  }
}
