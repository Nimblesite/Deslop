using System.Text.RegularExpressions;

namespace Examples.Validators
{
    // Regex-based email validator. Semantically equivalent to
    // EmailValidatorImperative. Completely different AST and tokens —
    // pure same-behavior, different-code cluster.
    public static class EmailValidatorRegex
    {
        private static readonly Regex Pattern = new Regex(
            @"^[A-Za-z0-9-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+$",
            RegexOptions.Compiled);

        public static bool IsValid(string? input)
        {
            if (string.IsNullOrWhiteSpace(input))
            {
                return false;
            }

            return Pattern.IsMatch(input);
        }
    }
}
