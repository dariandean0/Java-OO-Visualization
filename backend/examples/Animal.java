public abstract class Animal {
    protected String name;
    protected int age;

    public Animal(String name, int age) {
        this.name = name;
        this.age = age;
    }

    public abstract void makeSound();

    public void sleep() {
        System.out.println(name + " is sleeping");
    }

    public String getName() {
        return name;
    }

    public int getAge() {
        return age;
    }
}