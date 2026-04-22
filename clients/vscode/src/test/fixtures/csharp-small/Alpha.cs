public class Alpha {
    public int Compute(int x, int y) {
        var sum = x + y;
        var total = 0;
        for (var i = 0; i < 12; i++) {
            if (i % 2 == 0) {
                total += sum + i;
            } else {
                total += sum - i;
            }
        }
        if (total > 100) {
            total = total / 2;
        }
        return total * 2;
    }
}
