using System.Collections.Generic;
using System.IO;

namespace Examples.AsyncPatterns
{
    // Synchronous file processor. Blocks on every I/O call. Paired with
    // FileProcessorAsync (async/await) and FileProcessorTaskContinuation
    // (explicit Task.ContinueWith) — all three produce the same result
    // through very different control-flow shapes. Only the embedding
    // pass surfaces the same-behavior equivalence.
    public static class FileProcessorSync
    {
        public static IReadOnlyList<string> LoadLines(string path)
        {
            var lines = new List<string>();
            using var reader = new StreamReader(path);
            string? line;
            while ((line = reader.ReadLine()) != null)
            {
                if (!string.IsNullOrWhiteSpace(line))
                {
                    lines.Add(line.Trim());
                }
            }

            return lines;
        }

        public static int CountMatches(string path, string needle)
        {
            int matches = 0;
            using var reader = new StreamReader(path);
            string? line;
            while ((line = reader.ReadLine()) != null)
            {
                if (line.Contains(needle))
                {
                    matches = matches + 1;
                }
            }

            return matches;
        }
    }
}
