using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;

namespace Examples.AsyncPatterns
{
    // Task.ContinueWith twin of the above. Explicit callback style,
    // pre-async/await idiom. Same semantics as FileProcessorSync /
    // FileProcessorAsync.
    public static class FileProcessorTaskContinuation
    {
        public static Task<IReadOnlyList<string>> LoadLinesAsync(string path) =>
            Task.Run(() =>
            {
                var allLines = File.ReadAllLines(path);
                var filtered = allLines
                    .Where(line => !string.IsNullOrWhiteSpace(line))
                    .Select(line => line.Trim())
                    .ToList();
                return (IReadOnlyList<string>)filtered;
            });

        public static Task<int> CountMatchesAsync(string path, string needle) =>
            File.ReadAllLinesAsync(path)
                .ContinueWith(task => task.Result.Count(line => line.Contains(needle)));
    }
}
