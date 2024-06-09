import java.util.ArrayList;

public class Bench {
    public static void main (String[] args) {
        var n = 20;
        var buf = new ArrayList<String>();
        for (var a = 0; a < n; a++) {
            for (var b = 0; b < n; b++) {
                for (var c = 0; c < n; c++) {
                    for (var d = 0; d < n; d++) {
                        for (var e = 1; e < n; e++) {
                            buf.add(String.valueOf(c / e * d - b + a));
                        }
                    }
                }
            }
            System.out.print(a + 1);
            System.out.print(" / ");
            System.out.print(n);
            System.out.print(" done!\n");
        }
    }
}
