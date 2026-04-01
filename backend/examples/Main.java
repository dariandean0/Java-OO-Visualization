public class TestExecution {
  public static void main(String[] args) {
    Calculator calc = new Calculator();
    calc.add(5);
    calc.add(3);
    double result = calc.getResult();
    System.out.println(result);

    calc.clear();
    calc.add(12);
    calc.multiply(3);
    result = calc.getResult();
    System.out.println(result);
  }
}

public class Calculator {
  private double value;

  public Calculator() {
    this.value = 0.0;
  }

  public void clear() {
    this.value = 0.0;
  }

  public void add(double amount) {
    this.value += amount;
  }

  public void multiply(int n) {
    double initial_value = this.value;
    for (int i=1; i < n; i++) {
      this.add(initial_value);
    }
  }

  public double getResult() {
    return this.value;
  }
}
