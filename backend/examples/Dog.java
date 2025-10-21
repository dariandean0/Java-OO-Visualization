public class Dog extends Animal implements Trainable {
    private String breed;
    private boolean isTrained;

    public Dog(String name, int age, String breed) {
        super(name, age);
        this.breed = breed;
        this.isTrained = false;
    }

    @Override
    public void makeSound() {
        System.out.println("Woof! Woof!");
    }

    @Override
    public void train() {
        this.isTrained = true;
        System.out.println(getName() + " has been trained!");
    }

    @Override
    public boolean isLearnedSkill(String skill) {
        return isTrained && skill.equals("sit");
    }

    public void fetch() {
        if (isTrained) {
            System.out.println(getName() + " is fetching the ball!");
        } else {
            System.out.println(getName() + " doesn't know how to fetch yet.");
        }
    }

    public String getBreed() {
        return breed;
    }
}