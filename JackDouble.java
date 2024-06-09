/**
 * Wraps a {@code double} and provides methods for arithmatic.
 */
public class JackDouble {
    private double inner;

    private JackDouble(final double inner) {
        this.inner = inner;
    }

    public static JackDouble make(final double inner) {
        return new JackDouble(inner);
    }

    public static JackDouble make(final int inner) {
        return new JackDouble((double) inner);
    }

    public static JackDouble parse(final String s) {
        return new JackDouble(Double.parseDouble(s));
    }

    public JackDouble add(final JackDouble x) {
        return make(this.inner + x.inner);
    }

    public JackDouble sub(final JackDouble x) {
        return make(this.inner - x.inner);
    }

    public JackDouble mul(final JackDouble x) {
        return make(this.inner * x.inner);
    }

    public JackDouble div(final JackDouble x) {
        return make(this.inner / x.inner);
    }

    public JackDouble floor() {
        return make(Math.floor(inner));
    }

    public JackDouble ceil() {
        return make(Math.ceil(inner));
    }

    @Override
    public boolean equals(Object o) {
        if (o == this) {
            return true;
        }
        if (o.getClass() == JackDouble.class) {
            return this.inner == ((JackDouble) o).inner;
        } else {
            return false;
        }
    }

    @Override
    public int hashCode() {
        return Double.hashCode(inner);
    }

    @Override
    public String toString() {
        return Double.toString(inner);
    }
}
