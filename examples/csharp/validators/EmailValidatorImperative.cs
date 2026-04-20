namespace Examples.Validators
{
    // Imperative email validator. Walks the string character-by-character.
    // Semantically equivalent to EmailValidatorRegex and
    // EmailValidatorParser, but no structural or token signal can see that
    // — only an embedding pass surfaces the equivalence.
    public static class EmailValidatorImperative
    {
        public static bool IsValid(string? input)
        {
            if (string.IsNullOrWhiteSpace(input))
            {
                return false;
            }

            int atIndex = -1;
            for (int index = 0; index < input.Length; index = index + 1)
            {
                char ch = input[index];
                if (ch == '@')
                {
                    if (atIndex >= 0)
                    {
                        return false;
                    }
                    atIndex = index;
                }
            }

            if (atIndex <= 0 || atIndex >= input.Length - 1)
            {
                return false;
            }

            bool dotAfterAt = false;
            for (int index = atIndex + 1; index < input.Length; index = index + 1)
            {
                if (input[index] == '.')
                {
                    if (index == atIndex + 1 || index == input.Length - 1)
                    {
                        return false;
                    }
                    dotAfterAt = true;
                }
                else if (!char.IsLetterOrDigit(input[index]) && input[index] != '-')
                {
                    return false;
                }
            }

            return dotAfterAt;
        }
    }
}
