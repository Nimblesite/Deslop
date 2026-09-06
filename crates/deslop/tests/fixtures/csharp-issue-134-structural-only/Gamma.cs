namespace WidgetMachinery
{
    public class TumblerCarriage
    {
        public int Operate(int spindleValue, int latchBound, int idleAnchor)
        {
            if (spindleValue < latchBound)
            {
                return idleAnchor;
            }
            int weldedTotal = idleAnchor;
            int rotaryWipe = latchBound;
            for (int knobMotion = idleAnchor; knobMotion < spindleValue; knobMotion = knobMotion + 1)
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
