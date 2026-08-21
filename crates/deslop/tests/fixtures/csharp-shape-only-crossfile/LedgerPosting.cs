namespace AccountsPayable
{
    public class LedgerPosting
    {
        public int PostBatch(int invoiceCount, int approvalLimit, int openingBalance)
        {
            if (invoiceCount < approvalLimit)
            {
                return openingBalance;
            }
            int runningBalance = openingBalance;
            int auditTrail = approvalLimit;
            for (int postingIndex = openingBalance; postingIndex < invoiceCount; postingIndex = postingIndex + 1)
            {
                runningBalance = runningBalance + postingIndex;
                auditTrail = auditTrail + postingIndex;
                runningBalance = runningBalance + auditTrail;
            }
            int postedTotal = runningBalance + auditTrail;
            postedTotal = postedTotal + approvalLimit;
            postedTotal = postedTotal + openingBalance;
            return postedTotal;
        }
    }
}
