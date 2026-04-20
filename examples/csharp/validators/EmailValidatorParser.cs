using System.Linq;

namespace Examples.Validators
{
    // Parser-style email validator. Splits on '@' and '.' and checks each
    // segment. Semantically equivalent to the imperative and regex
    // variants — only the embedding pass catches this as a Type-4
    // clone.
    public static class EmailValidatorParser
    {
        public static bool IsValid(string? input)
        {
            if (string.IsNullOrWhiteSpace(input))
            {
                return false;
            }

            var parts = input.Split('@');
            if (parts.Length != 2)
            {
                return false;
            }

            var local = parts[0];
            var domain = parts[1];
            if (local.Length == 0 || domain.Length == 0)
            {
                return false;
            }

            var domainSegments = domain.Split('.');
            if (domainSegments.Length < 2)
            {
                return false;
            }

            if (domainSegments.Any(segment => segment.Length == 0))
            {
                return false;
            }

            return domainSegments.All(segment =>
                segment.All(ch => char.IsLetterOrDigit(ch) || ch == '-'));
        }
    }
}
