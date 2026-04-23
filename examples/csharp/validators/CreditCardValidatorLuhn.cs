namespace Examples.Validators
{
    // Classic Luhn check — imperative implementation. Paired with
    // CreditCardValidatorFunctional below for a same behavior,
    // different code [Type-4] cluster.
    public static class CreditCardValidatorLuhn
    {
        public static bool IsValid(string? number)
        {
            if (string.IsNullOrWhiteSpace(number))
            {
                return false;
            }

            int total = 0;
            bool doubling = false;
            for (int index = number.Length - 1; index >= 0; index = index - 1)
            {
                char ch = number[index];
                if (!char.IsDigit(ch))
                {
                    return false;
                }

                int digit = ch - '0';
                if (doubling)
                {
                    digit = digit * 2;
                    if (digit > 9)
                    {
                        digit = digit - 9;
                    }
                }

                total = total + digit;
                doubling = !doubling;
            }

            return total % 10 == 0;
        }
    }
}
