public class Beta {
    public int Compute(int a, int b) {
        var sum = a + b;
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
