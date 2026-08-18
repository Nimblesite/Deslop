namespace MixedConcerns
{
    public class Scaffold
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

        public int Operate(int spindleValue, int latchBound, int idleAnchor)
        {
            if (spindleValue < latchBound)
            {
                return idleAnchor;
            }
            int weldedTotal = idleAnchor;
            int rotaryWipe = latchBound;
            for (int knobMotion = idleAnchor; knobMotion < spindleValue; knobMotion = knobMotion + 3)
            {
                weldedTotal = weldedTotal + knobMotion;
                rotaryWipe = rotaryWipe + knobMotion;
                weldedTotal = weldedTotal + rotaryWipe;
            }
            int finalReading = weldedTotal + rotaryWipe;
            finalReading = finalReading + latchBound;
            finalReading = finalReading + idleAnchor;
            return finalReading;
        }
    }
}
