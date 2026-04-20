using System.Collections.Generic;
using System.IO;
using System.Threading.Tasks;

namespace Examples.AsyncPatterns
{
    // async/await twin of FileProcessorSync. Same semantics, totally
    // different AST (state machine desugared by the compiler).
    public static class FileProcessorAsync
    {
        public static async Task<IReadOnlyList<string>> LoadLinesAsync(string path)
        {
            var lines = new List<string>();
            using var reader = new StreamReader(path);
            string? line;
            while ((line = await reader.ReadLineAsync()) != null)
            {
                if (!string.IsNullOrWhiteSpace(line))
                {
                    lines.Add(line.Trim());
                }
            }

            return lines;
        }

        public static async Task<int> CountMatchesAsync(string path, string needle)
        {
            int matches = 0;
            using var reader = new StreamReader(path);
            string? line;
            while ((line = await reader.ReadLineAsync()) != null)
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
