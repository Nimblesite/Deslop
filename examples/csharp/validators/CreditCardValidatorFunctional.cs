using System.Linq;

namespace Examples.Validators
{
    // Functional Luhn check — the same Luhn algorithm expressed via LINQ
    // `Select` + `Sum`. Tokens and AST are completely different from the
    // imperative version; embedding similarity is very high.
    public static class CreditCardValidatorFunctional
    {
        public static bool IsValid(string? number)
        {
            if (string.IsNullOrWhiteSpace(number) || !number.All(char.IsDigit))
            {
                return false;
            }

            var total = number
                .Reverse()
                .Select((ch, position) => (digit: ch - '0', position))
                .Sum(pair => pair.position % 2 == 1
                    ? (pair.digit * 2 > 9 ? pair.digit * 2 - 9 : pair.digit * 2)
                    : pair.digit);

            return total % 10 == 0;
        }
    }
}
